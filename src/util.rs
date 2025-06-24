//! Misc helpers (feature gates, doc_cfg, etc.).

/// Conditionally add `#[doc(cfg(feature = $feat))]`.
#[macro_export]
macro_rules! doc_cfg {
    ($feat:literal, $item:item) => {
        #[cfg_attr(docsrs, doc(cfg(feature = $feat)))]
        $item
    };
}
