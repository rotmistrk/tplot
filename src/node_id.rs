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

    /// Resolve a dotted path to a filesystem path using names.toml at each level.
    /// Named segments are looked up in the names.toml of their parent directory.
    pub(crate) fn resolve(&self, nodes_root: &std::path::Path) -> Option<std::path::PathBuf> {
        let mut current = nodes_root.to_path_buf();
        for seg in &self.segments {
            match seg {
                Segment::Num(n) => {
                    current.push(format!("{n:03}"));
                }
                Segment::Name(name) => {
                    let names_file = current.join("names.toml");
                    let mapping = read_names(&names_file)?;
                    let num = mapping.get(name.as_str())?;
                    current.push(format!("{num:03}"));
                }
            }
            if !current.is_dir() {
                return None;
            }
        }
        Some(current)
    }

    /// Convert to filesystem path without name resolution (numeric segments only).
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

/// Read names.toml: maps logical name → numeric index.
fn read_names(path: &std::path::Path) -> Option<std::collections::HashMap<String, u32>> {
    let content = std::fs::read_to_string(path).ok()?;
    let table: toml::Table = toml::from_str(&content).ok()?;
    let mut map = std::collections::HashMap::new();
    for (key, val) in table {
        if let toml::Value::Integer(n) = val {
            map.insert(key, n as u32);
        }
    }
    Some(map)
}

/// Assign a logical name to a numeric node at the given parent directory.
#[allow(dead_code)]
pub(crate) fn assign_name(parent_dir: &std::path::Path, name: &str, num: u32) -> Result<(), String> {
    let names_file = parent_dir.join("names.toml");
    let mut table: toml::Table = if names_file.exists() {
        let content = std::fs::read_to_string(&names_file).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())?
    } else {
        toml::Table::new()
    };
    table.insert(name.to_string(), toml::Value::Integer(num as i64));
    let out = toml::to_string_pretty(&table).map_err(|e| e.to_string())?;
    std::fs::write(&names_file, out).map_err(|e| e.to_string())?;
    Ok(())
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

    #[test]
    fn test_name_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let nodes = dir.path();

        // Create nodes/001/000/
        std::fs::create_dir_all(nodes.join("001/000")).unwrap();

        // Assign name "base" → 1 at root level
        super::assign_name(nodes, "base", 1).unwrap();
        // Assign name "tcp" → 0 inside 001/
        super::assign_name(&nodes.join("001"), "tcp", 0).unwrap();

        // Resolve "base.tcp" → nodes/001/000
        let id = NodeId::parse("base.tcp");
        let resolved = id.resolve(nodes).unwrap();
        assert_eq!(resolved, nodes.join("001/000"));
    }
}
