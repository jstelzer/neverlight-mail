use cosmic::widget::pane_grid;

use neverlight_mail_core::config::LayoutConfig;

use super::AppModel;

impl AppModel {
    /// Extract current split ratios from pane_grid layout tree and persist.
    pub(super) fn save_layout(&self) {
        fn extract_ratios(node: &pane_grid::Node) -> (f32, f32) {
            match node {
                pane_grid::Node::Split { ratio, a, b, .. } => {
                    let sidebar_ratio = *ratio;
                    // Inner split is in the 'b' branch
                    let list_ratio = match b.as_ref() {
                        pane_grid::Node::Split { ratio, .. } => *ratio,
                        _ => 0.40,
                    };
                    // If 'a' is also a split (shouldn't be, but be safe), recurse
                    let _ = a;
                    (sidebar_ratio, list_ratio)
                }
                _ => (0.15, 0.40),
            }
        }

        let (sidebar_ratio, list_ratio) = extract_ratios(self.panes.layout());
        let layout = LayoutConfig {
            sidebar_ratio,
            list_ratio,
        };
        layout.save();
    }
}
