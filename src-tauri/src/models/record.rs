use serde::{Deserialize, Serialize};

/// 内容类型枚举
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Image,
    Html,
    Link,
}

/// 剪贴板记录结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardRecord {
    pub id: i64,                     // 记录ID
    pub content_type: ContentType,   // 内容类型 (强类型)
    pub content: String,             // 文本内容
    pub image_data: Option<Vec<u8>>, // 图片数据
    pub created_at: String,          // 创建时间
    pub is_favorite: bool,           // 是否收藏
    pub is_pinned: bool,             // 是否置顶
}
