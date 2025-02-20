use crate::private_hack::*;

#[derive(Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct WidgetHits {
    pub close: Vec<crate::private_hack::widget_rect::WidgetRect>,
    pub contains_pointer: Vec<crate::private_hack::widget_rect::WidgetRect>,
    pub click: Option<crate::private_hack::widget_rect::WidgetRect>,
    pub drag: Option<crate::private_hack::widget_rect::WidgetRect>,
}
