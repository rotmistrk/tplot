//! Node ID — hierarchical dotted path with optional logical names.

/// A node's hierarchical identity: e.g. "1.0.2" or "1.base.2"
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NodeId {
    segments: Vec<Segment>,
}

/// A single path segment: numeric auto-assigned or user-named.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Segment {
    Num(u32),
    Name(String),
}

impl NodeId {
    /// Parse a dotted path string into a NodeId.
    pub(crate) fn parse(s: &str) -> Self {
        let segments = s
            .split('.')
            .map(|part| {
                part.parse::<u32>()
                    .map(Segment::Num)
                    .unwrap_or_else(|_| Segment::Name(part.to_string()))
            })
            .collect();
        Self { segments }
    }

    /// Create a root node id (just a number).
    pub(crate) fn root(n: u32) -> Self {
        Self {
            segments: vec![Segment::Num(n)],
        }
    }

    /// Create a child by appending a numeric segment.
    pub(crate) fn child(&self, n: u32) -> Self {
        let mut segs = self.segments.clone();
        segs.push(Segment::Num(n));
        Self { segments: segs }
    }

    /// Create a child by appending a named segment.
    pub(crate) fn child_named(&self, name: &str) -> Self {
        let mut segs = self.segments.clone();
        segs.push(Segment::Name(name.to_string()));
        Self { segments: segs }
    }

    /// Get the parent id (all segments except last), or None for roots.
    pub(crate) fn parent(&self) -> Option<Self> {
        if self.segments.len() <= 1 {
            return None;
        }
        Some(Self {
            segments: self.segments[..self.segments.len() - 1].to_vec(),
        })
    }

    /// Depth (0 for root, 1 for first-level child, etc.)
    pub(crate) fn depth(&self) -> usize {
        self.segments.len() - 1
    }

    /// Convert to filesystem-safe directory path (nested, 0-padded).
    pub(crate) fn to_dir_path(&self) -> String {
        self.segments
            .iter()
            .map(|s| match s {
                Segment::Num(n) => format!("{n:03}"),
                Segment::Name(name) => name.clone(),
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Display as dotted string.
    pub(crate) fn to_dotted(&self) -> String {
        self.segments
            .iter()
            .map(|s| match s {
                Segment::Num(n) => n.to_string(),
                Segment::Name(name) => name.clone(),
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_dotted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_numeric() {
        let id = NodeId::parse("1.0.2");
        assert_eq!(id.segments.len(), 3);
        assert_eq!(id.to_dotted(), "1.0.2");
        assert_eq!(id.to_dir_path(), "001/000/002");
    }

    #[test]
    fn test_parse_named() {
        let id = NodeId::parse("1.base.2");
        assert_eq!(id.to_dotted(), "1.base.2");
        assert_eq!(id.to_dir_path(), "001/base/002");
    }

    #[test]
    fn test_hierarchy() {
        let root = NodeId::root(1);
        assert_eq!(root.depth(), 0);
        assert!(root.parent().is_none());

        let child = root.child(0);
        assert_eq!(child.to_dotted(), "1.0");
        assert_eq!(child.depth(), 1);
        assert_eq!(child.parent().unwrap().to_dotted(), "1");

        let named = root.child_named("base");
        assert_eq!(named.to_dotted(), "1.base");

        let grandchild = named.child(1);
        assert_eq!(grandchild.to_dotted(), "1.base.1");
        assert_eq!(grandchild.to_dir_path(), "001/base/001");
    }
}
