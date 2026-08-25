use super::record::ContentType;
use super::settings::Settings;
use serde::{Deserialize, Serialize};

/// 收藏导出项结构
#[derive(Debug, Serialize, Deserialize)]
pub struct FavoriteTransferItem {
    pub content_type: ContentType, // 内容类型
    pub content: String,           // 内容
    pub is_pinned: bool,           // 是否置顶
}

/// 数据导出包结构（包含收藏和设置）
#[derive(Debug, Serialize, Deserialize)]
pub struct FavoriteTransferPackage {
    pub favorites: Vec<FavoriteTransferItem>, // 收藏列表
    pub settings: Settings,                   // 应用设置
}

/// 收藏导出结果
#[derive(Debug, Serialize, Deserialize)]
pub struct FavoriteExportResult {
    pub count: i32,   // 导出数量
    pub path: String, // 文件路径
}
