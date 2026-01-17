// ============================================================================
// HTML/SVG/MathML Element Validation Constants
// ============================================================================

/// Standard HTML elements (for validation)
pub const STANDARD_HTML_ELEMENTS: &[&str] = &[
    // Document metadata
    "base", "head", "link", "meta", "style", "title",
    // Sectioning
    "body", "address", "article", "aside", "footer", "header",
    "h1", "h2", "h3", "h4", "h5", "h6", "hgroup", "main", "nav", "section", "search",
    // Text content
    "blockquote", "dd", "div", "dl", "dt", "figcaption", "figure", "hr", "li", "menu", "ol", "p", "pre", "ul",
    // Inline text semantics
    "a", "abbr", "b", "bdi", "bdo", "br", "cite", "code", "data", "dfn", "em", "i", "kbd", "mark", "q",
    "rp", "rt", "ruby", "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var", "wbr",
    // Image and multimedia
    "area", "audio", "img", "map", "track", "video",
    // Embedded content
    "embed", "iframe", "object", "param", "picture", "portal", "source",
    // SVG and MathML (container elements only - child elements listed separately)
    "svg", "math",
    // Scripting
    "canvas", "noscript", "script",
    // Edits
    "del", "ins",
    // Table content
    "caption", "col", "colgroup", "table", "tbody", "td", "tfoot", "th", "thead", "tr",
    // Forms
    "button", "datalist", "fieldset", "form", "input", "label", "legend", "meter",
    "optgroup", "option", "output", "progress", "select", "textarea",
    // Interactive elements
    "details", "dialog", "summary",
    // Web Components
    "slot", "template",
];

/// SVG elements (child elements of <svg>)
pub const SVG_ELEMENTS: &[&str] = &[
    // Shape elements
    "circle", "ellipse", "line", "path", "polygon", "polyline", "rect",
    // Container elements
    "a", "defs", "g", "marker", "mask", "pattern", "svg", "switch", "symbol",
    // Gradient elements
    "linearGradient", "radialGradient", "stop",
    // Text elements
    "text", "textPath", "tspan",
    // Descriptive elements
    "desc", "metadata", "title",
    // Clipping and masking
    "clipPath",
    // Other elements
    "foreignObject", "image", "use", "view",
    // Filter elements
    "feBlend", "feColorMatrix", "feComponentTransfer", "feComposite",
    "feConvolveMatrix", "feDiffuseLighting", "feDisplacementMap",
    "feDistantLight", "feDropShadow", "feFlood", "feFuncA", "feFuncB",
    "feFuncG", "feFuncR", "feGaussianBlur", "feImage", "feMerge",
    "feMergeNode", "feMorphology", "feOffset", "fePointLight",
    "feSpecularLighting", "feSpotLight", "feTile", "feTurbulence", "filter",
    // Animation elements
    "animate", "animateMotion", "animateTransform", "mpath", "set",
];

/// MathML elements (child elements of <math>)
pub const MATHML_ELEMENTS: &[&str] = &[
    // Token elements
    "mi", "mn", "mo", "ms", "mtext", "mspace", "mglyph",
    // General layout
    "mrow", "mfrac", "msqrt", "mroot", "mstyle", "merror",
    "mpadded", "mphantom", "mfenced", "menclose",
    // Script and limit elements
    "msub", "msup", "msubsup", "munder", "mover", "munderover", "mmultiscripts",
    // Tabular math
    "mtable", "mtr", "mtd", "maligngroup", "malignmark", "mlabeledtr",
    // Elementary math
    "mstack", "mlongdiv", "msgroup", "msrow", "mscarries", "mscarry", "msline",
    // Other elements
    "maction", "semantics", "annotation", "annotation-xml",
];

/// Capitalize the first letter of a string (for suggesting PascalCase component names)
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}
