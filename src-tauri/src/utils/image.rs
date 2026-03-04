use image::{codecs::png::PngEncoder, ColorType, ImageEncoder, ImageFormat};
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

// =============================================================================
// 图片处理安全限制常量 (Safety Constraints)
// =============================================================================
/// 原始 RGBA 数据（解压后驻留在内存中）的上限：48 MB
pub const MAX_IMAGE_BYTES: usize = 48 * 1024 * 1024;
/// 像素总数物理上限，防内存炸弹（约等于 1920x1350）
pub const MAX_IMAGE_PIXELS: usize = 2_600_000;
/// 强制限宽/高的单边最大尺寸
pub const MAX_IMAGE_DIMENSION: usize = 2200;
/// 编码后写入本地存储库的单张 PNG 上限：6 MB
pub const MAX_ENCODED_IMAGE_BYTES: usize = 6 * 1024 * 1024;

/// 将原始 RGBA 缓冲流按照安全参数编码为标准的 PNG 图片格式序列。
/// 返回 `Result<Vec<u8>, String>` 避免对自定义 Error 的过度依赖。
pub fn encode_rgba_to_png(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let encoder = PngEncoder::new(&mut out);
    encoder
        .write_image(rgba, width as u32, height as u32, ColorType::Rgba8.into())
        .map_err(|e| format!("PNG编码失败: {}", e))?;
    Ok(out)
}

/// 纯算法：使用最近邻近似法 (Nearest Neighbor) 将超大 RGBA 图片硬缩小，以适应存储或展示限制
pub fn downscale_rgba_nearest(
    width: usize,
    height: usize,
    rgba: &[u8],
    target_width: usize,
    target_height: usize,
) -> Vec<u8> {
    let mut out = vec![0u8; target_width * target_height * 4];
    for ty in 0..target_height {
        let sy = ty * height / target_height;
        for tx in 0..target_width {
            let sx = tx * width / target_width;
            let src = (sy * width + sx) * 4;
            let dst = (ty * target_width + tx) * 4;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    out
}

/// 标准化图形数据：保证存入数据库前的高宽不要超过系统阈值。
/// 返回值 Cow (Clone On Write) 是一个极为优雅的解法：
/// 如果未发生缩放，它直接以 Zero-Copy（零拷贝）的特性返回传入的底层引用 (Borrowed)；
/// 而发生了缩放时才会在堆上创建新的图形字节码 (Owned)
pub fn normalize_image_for_storage<'a>(
    width: usize,
    height: usize,
    rgba: &'a [u8],
) -> (usize, usize, Cow<'a, [u8]>, bool) {
    let mut ratio: f64 = 1.0;

    // 如果任意单边超出了界限，计算缩小比率
    if width > MAX_IMAGE_DIMENSION {
        ratio = ratio.max(width as f64 / MAX_IMAGE_DIMENSION as f64);
    }
    if height > MAX_IMAGE_DIMENSION {
        ratio = ratio.max(height as f64 / MAX_IMAGE_DIMENSION as f64);
    }

    // 如果总像素超越限制，按照根号比率降维缩放
    let pixels = width.saturating_mul(height);
    if pixels > MAX_IMAGE_PIXELS {
        ratio = ratio.max((pixels as f64 / MAX_IMAGE_PIXELS as f64).sqrt());
    }

    // 免分配直接归还
    if ratio <= 1.0 {
        return (width, height, Cow::Borrowed(rgba), false);
    }

    let target_width = ((width as f64 / ratio).round() as usize).max(1);
    let target_height = ((height as f64 / ratio).round() as usize).max(1);

    // 强制缩放截取
    let resized = downscale_rgba_nearest(width, height, rgba, target_width, target_height);
    (target_width, target_height, Cow::Owned(resized), true)
}

/// 基于图片指纹生成唯一字符串哈希，为后续实现**图片秒传与剪贴板去重**服务。
/// 特点：并不对几十 MB 的图片全量扫描（耗时巨长），而是两头各自抽样抓取 4KB 数据组合特征指纹
pub fn image_signature(width: usize, height: usize, rgba: &[u8]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    rgba.len().hash(&mut hasher);

    // 掐头采样 4KB
    for b in rgba.iter().take(4096) {
        b.hash(&mut hasher);
    }
    // 去尾采样 4KB
    for b in rgba.iter().rev().take(4096) {
        b.hash(&mut hasher);
    }

    format!("image:{}:{}:{}", width, height, hasher.finish())
}

/// 从 PNG 二进制流逆向渲染回 RGBA 矩阵，用于加载已存活的冷归档图片
pub fn decode_png_rgba(png_bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    let image = image::load_from_memory_with_format(png_bytes, ImageFormat::Png)
        .map_err(|e| format!("加载PNG失败: {}", e))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((width as usize, height as usize, rgba.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_rgba_nearest_selects_top_left_pixel_for_one_by_one() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, // row 0
            0, 0, 255, 255, 255, 255, 255, 255, // row 1
        ];
        let out = downscale_rgba_nearest(2, 2, &rgba, 1, 1);
        assert_eq!(out, vec![255, 0, 0, 255]);
    }

    #[test]
    fn normalize_image_for_storage_keeps_small_image() {
        let rgba = vec![42u8; 100 * 100 * 4];
        let (w, h, bytes, scaled) = normalize_image_for_storage(100, 100, &rgba);
        assert_eq!((w, h), (100, 100));
        assert!(!scaled);
        assert_eq!(bytes.as_ref(), rgba.as_slice());
    }

    #[test]
    fn normalize_image_for_storage_scales_large_image() {
        let width = MAX_IMAGE_DIMENSION + 200;
        let height = MAX_IMAGE_DIMENSION + 100;
        let rgba = vec![7u8; width * height * 4];

        let (w, h, bytes, scaled) = normalize_image_for_storage(width, height, &rgba);
        assert!(scaled);
        assert!(w <= MAX_IMAGE_DIMENSION);
        assert!(h <= MAX_IMAGE_DIMENSION);
        assert_eq!(bytes.len(), w * h * 4);
    }

    #[test]
    fn encode_decode_png_round_trip_dimensions() {
        let width = 3;
        let height = 2;
        let rgba = vec![
            1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255, 13, 14, 15, 255, 16, 17, 18,
            255,
        ];

        let png = encode_rgba_to_png(width, height, &rgba).expect("encode png");
        let (decoded_w, decoded_h, decoded_rgba) = decode_png_rgba(&png).expect("decode png");
        assert_eq!((decoded_w, decoded_h), (width, height));
        assert_eq!(decoded_rgba, rgba);
    }
}
