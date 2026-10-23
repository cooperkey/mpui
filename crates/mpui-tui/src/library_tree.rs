use std::collections::HashSet;
use std::path::{Path, PathBuf};

use mpui_core::library::Library;
use mpui_core::track::TrackId;

#[derive(Debug, Clone)]
pub struct LibraryTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub track_id: Option<TrackId>,
    pub expanded: bool,
    pub children: Vec<LibraryTreeNode>,
}

#[derive(Debug, Clone)]
pub struct VisibleTreeItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub expanded: bool,
    pub depth: usize,
    pub track_id: Option<TrackId>,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryTree {
    pub roots: Vec<LibraryTreeNode>,
    pub visible_items: Vec<VisibleTreeItem>,
    pub expanded_paths: HashSet<PathBuf>,
}

impl LibraryTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the file tree from tracks present in the library.
    pub fn build(&mut self, library: &Library, directories: &[PathBuf]) {
        let mut tracks: Vec<(TrackId, PathBuf, String)> = library
            .track_ids()
            .iter()
            .filter_map(|&id| {
                library.get(id).map(|t| {
                    let title = t.display_title().to_string();
                    (id, t.path.clone(), title)
                })
            })
            .collect();

        if tracks.is_empty() {
            self.roots.clear();
            self.visible_items.clear();
            return;
        }

        // Sort by path for consistent tree construction
        tracks.sort_by(|a, b| a.1.cmp(&b.1));

        // Determine base directories or common prefixes
        let expanded_dirs: Vec<PathBuf> = directories
            .iter()
            .map(|d| mpui_config::expand_tilde(d))
            .collect();

        let common = find_common_parent(&tracks);

        let mut roots: Vec<LibraryTreeNode> = Vec::new();

        for (id, path, display_name) in tracks {
            let matched_base = expanded_dirs.iter().find(|d| path.starts_with(d));
            let actual_base = if let Some(base) = matched_base {
                Some(base.clone())
            } else if let Some(ref c) = common {
                Some(c.parent().unwrap_or(c).to_path_buf())
            } else {
                None
            };

            let rel_path = if let Some(ref base) = actual_base {
                path.strip_prefix(base).unwrap_or(&path).to_path_buf()
            } else {
                path.clone()
            };

            let components: Vec<String> = rel_path
                .components()
                .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
                .collect();

            if components.is_empty() {
                continue;
            }

            let mut current_nodes = &mut roots;
            let mut accumulated_path = actual_base.unwrap_or_default();

            for (i, comp) in components.iter().enumerate() {
                let is_last = i == components.len() - 1;
                accumulated_path = accumulated_path.join(comp);

                if is_last {
                    // File leaf node
                    current_nodes.push(LibraryTreeNode {
                        name: display_name.clone(),
                        path: path.clone(),
                        is_dir: false,
                        track_id: Some(id),
                        expanded: false,
                        children: Vec::new(),
                    });
                } else {
                    // Directory branch node
                    let pos = current_nodes.iter().position(|n| n.is_dir && n.name == *comp);
                    let idx = match pos {
                        Some(p) => p,
                        None => {
                            let is_expanded = self.expanded_paths.contains(&accumulated_path)
                                || self.expanded_paths.is_empty(); // default expand top-level
                            current_nodes.push(LibraryTreeNode {
                                name: comp.clone(),
                                path: accumulated_path.clone(),
                                is_dir: true,
                                track_id: None,
                                expanded: is_expanded,
                                children: Vec::new(),
                            });
                            current_nodes.len() - 1
                        }
                    };
                    current_nodes = &mut current_nodes[idx].children;
                }
            }
        }

        // Sort children: directories first (alphabetically), then tracks
        sort_tree_nodes(&mut roots);

        // Store initially expanded paths if this was first build
        if self.expanded_paths.is_empty() {
            collect_expanded_paths(&roots, &mut self.expanded_paths);
        }

        self.roots = roots;
        self.rebuild_visible();
    }

    /// Flatten the visible (expanded) nodes in the tree into a list.
    pub fn rebuild_visible(&mut self) {
        let mut items = Vec::new();
        for root in &self.roots {
            self.flatten_node(root, 0, &mut items);
        }
        self.visible_items = items;
    }

    fn flatten_node(&self, node: &LibraryTreeNode, depth: usize, out: &mut Vec<VisibleTreeItem>) {
        out.push(VisibleTreeItem {
            name: node.name.clone(),
            path: node.path.clone(),
            is_dir: node.is_dir,
            expanded: node.expanded,
            depth,
            track_id: node.track_id,
        });
        if node.is_dir && node.expanded {
            for child in &node.children {
                self.flatten_node(child, depth + 1, out);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.visible_items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visible_items.is_empty()
    }

    pub fn get_track_id(&self, index: usize) -> Option<TrackId> {
        self.visible_items.get(index).and_then(|item| {
            if let Some(id) = item.track_id {
                Some(id)
            } else if item.is_dir {
                // Return first track under directory if selecting directory
                self.find_first_track_in_dir(&item.path)
            } else {
                None
            }
        })
    }

    fn find_first_track_in_dir(&self, dir_path: &Path) -> Option<TrackId> {
        fn search_nodes(nodes: &[LibraryTreeNode], target: &Path) -> Option<TrackId> {
            for node in nodes {
                if node.is_dir && node.path == target {
                    return first_track(node);
                }
                if node.is_dir {
                    if let Some(id) = search_nodes(&node.children, target) {
                        return Some(id);
                    }
                }
            }
            None
        }

        fn first_track(node: &LibraryTreeNode) -> Option<TrackId> {
            if let Some(id) = node.track_id {
                return Some(id);
            }
            for child in &node.children {
                if let Some(id) = first_track(child) {
                    return Some(id);
                }
            }
            None
        }

        search_nodes(&self.roots, dir_path)
    }

    /// Toggle expand/collapse state for visible item at index. Returns true if toggled.
    pub fn toggle_expand(&mut self, index: usize) -> bool {
        if let Some(item) = self.visible_items.get(index).cloned() {
            if item.is_dir {
                let new_state = !item.expanded;
                self.set_node_expanded(&item.path, new_state);
                if new_state {
                    self.expanded_paths.insert(item.path);
                } else {
                    self.expanded_paths.remove(&item.path);
                }
                self.rebuild_visible();
                return true;
            }
        }
        false
    }

    /// Expand folder at visible index.
    pub fn expand(&mut self, index: usize) -> bool {
        if let Some(item) = self.visible_items.get(index).cloned() {
            if item.is_dir && !item.expanded {
                self.set_node_expanded(&item.path, true);
                self.expanded_paths.insert(item.path);
                self.rebuild_visible();
                return true;
            }
        }
        false
    }

    /// Collapse folder at visible index, or move cursor to parent directory.
    pub fn collapse(&mut self, index: usize) -> Option<usize> {
        let item = self.visible_items.get(index)?.clone();

        if item.is_dir && item.expanded {
            self.set_node_expanded(&item.path, false);
            self.expanded_paths.remove(&item.path);
            self.rebuild_visible();
            return Some(index);
        }

        // If collapsed directory or file: jump to parent directory
        if let Some(parent) = item.path.parent() {
            for (idx, v) in self.visible_items.iter().enumerate() {
                if v.is_dir && v.path == parent {
                    return Some(idx);
                }
            }
        }

        None
    }

    /// Reveal track in tree: expands all ancestor directories and returns its visible index.
    pub fn reveal_track(&mut self, track_id: TrackId) -> Option<usize> {
        let mut ancestors = Vec::new();
        if self.find_track_ancestors(&self.roots, track_id, &mut ancestors) {
            for anc in &ancestors {
                self.expanded_paths.insert(anc.clone());
            }
            Self::expand_ancestor_nodes(&mut self.roots, &ancestors);
            self.rebuild_visible();

            return self
                .visible_items
                .iter()
                .position(|item| item.track_id == Some(track_id));
        }
        None
    }

    fn find_track_ancestors(
        &self,
        nodes: &[LibraryTreeNode],
        track_id: TrackId,
        ancestors: &mut Vec<PathBuf>,
    ) -> bool {
        for node in nodes {
            if node.track_id == Some(track_id) {
                return true;
            }
            if node.is_dir {
                ancestors.push(node.path.clone());
                if self.find_track_ancestors(&node.children, track_id, ancestors) {
                    return true;
                }
                ancestors.pop();
            }
        }
        false
    }

    fn expand_ancestor_nodes(nodes: &mut [LibraryTreeNode], ancestors: &[PathBuf]) {
        for node in nodes.iter_mut() {
            if node.is_dir && ancestors.contains(&node.path) {
                node.expanded = true;
                Self::expand_ancestor_nodes(&mut node.children, ancestors);
            }
        }
    }

    fn set_node_expanded(&mut self, path: &Path, expanded: bool) {
        fn update(nodes: &mut [LibraryTreeNode], path: &Path, expanded: bool) {
            for node in nodes.iter_mut() {
                if node.is_dir {
                    if node.path == path {
                        node.expanded = expanded;
                        return;
                    }
                    update(&mut node.children, path, expanded);
                }
            }
        }
        update(&mut self.roots, path, expanded);
    }

    /// Collect all track IDs contained in the subtree at visible index.
    pub fn collect_track_ids(&self, index: usize) -> Vec<TrackId> {
        let mut result = Vec::new();
        if let Some(item) = self.visible_items.get(index) {
            if let Some(id) = item.track_id {
                result.push(id);
            } else if item.is_dir {
                self.collect_under_dir(&self.roots, &item.path, &mut result);
            }
        }
        result
    }

    fn collect_under_dir(
        &self,
        nodes: &[LibraryTreeNode],
        target_dir: &Path,
        out: &mut Vec<TrackId>,
    ) {
        for node in nodes {
            if node.is_dir && node.path == target_dir {
                Self::collect_all_tracks(node, out);
                return;
            }
            if node.is_dir {
                self.collect_under_dir(&node.children, target_dir, out);
            }
        }
    }

    fn collect_all_tracks(node: &LibraryTreeNode, out: &mut Vec<TrackId>) {
        if let Some(id) = node.track_id {
            out.push(id);
        }
        for child in &node.children {
            Self::collect_all_tracks(child, out);
        }
    }
}

fn sort_tree_nodes(nodes: &mut [LibraryTreeNode]) {
    nodes.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    for node in nodes.iter_mut() {
        if node.is_dir {
            sort_tree_nodes(&mut node.children);
        }
    }
}

fn collect_expanded_paths(nodes: &[LibraryTreeNode], out: &mut HashSet<PathBuf>) {
    for node in nodes {
        if node.is_dir && node.expanded {
            out.insert(node.path.clone());
            collect_expanded_paths(&node.children, out);
        }
    }
}

fn find_common_parent(tracks: &[(TrackId, PathBuf, String)]) -> Option<PathBuf> {
    if tracks.is_empty() {
        return None;
    }

    let mut common = tracks[0].1.parent()?.to_path_buf();
    for (_, path, _) in &tracks[1..] {
        while !path.starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    Some(common)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpui_core::track::Track;

    fn create_test_track(id: u64, path: &str, title: &str) -> Track {
        Track {
            id: TrackId(id),
            path: PathBuf::from(path),
            title: Some(title.to_string()),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
            album_artist: None,
            genre: None,
            year: None,
            track_number: None,
            disc_number: None,
            duration: Some(std::time::Duration::from_secs(200)),
            sample_rate: None,
            channels: None,
            bitrate: None,
            format: "mp3".to_string(),
            replay_gain_track: None,
            replay_gain_album: None,
        }
    }

    #[test]
    fn test_tree_build_from_paths() {
        let mut lib = Library::new();
        lib.insert(create_test_track(1, "/music/Rock/Queen/01.mp3", "Bohemian Rhapsody"));
        lib.insert(create_test_track(2, "/music/Rock/Queen/02.mp3", "Radio Ga Ga"));
        lib.insert(create_test_track(3, "/music/Jazz/Miles/01.mp3", "So What"));

        let mut tree = LibraryTree::new();
        tree.build(&lib, &[PathBuf::from("/music")]);

        assert!(!tree.is_empty());
        // Top level should have Jazz and Rock directories
        assert!(tree.visible_items.iter().any(|item| item.name == "Rock" && item.is_dir));
        assert!(tree.visible_items.iter().any(|item| item.name == "Jazz" && item.is_dir));
    }

    #[test]
    fn test_tree_expand_collapse() {
        let mut lib = Library::new();
        lib.insert(create_test_track(1, "/music/Album/01.mp3", "Song 1"));
        lib.insert(create_test_track(2, "/music/Album/02.mp3", "Song 2"));

        let mut tree = LibraryTree::new();
        tree.build(&lib, &[PathBuf::from("/music")]);

        // Initially Album is expanded
        let album_idx = tree.visible_items.iter().position(|i| i.name == "Album").unwrap();
        assert!(tree.visible_items[album_idx].expanded);
        let count_expanded = tree.len();

        // Collapse Album
        assert_eq!(tree.collapse(album_idx), Some(album_idx));
        assert!(!tree.visible_items[album_idx].expanded);
        assert_eq!(tree.len(), 1); // Only Album directory is visible

        // Expand Album again
        assert!(tree.expand(album_idx));
        assert_eq!(tree.len(), count_expanded);
    }

    #[test]
    fn test_tree_reveal_track() {
        let mut lib = Library::new();
        lib.insert(create_test_track(1, "/music/A/B/C/song.mp3", "Deep Song"));

        let mut tree = LibraryTree::new();
        tree.build(&lib, &[PathBuf::from("/music")]);

        // Collapse all directories
        tree.expanded_paths.clear();
        for root in &mut tree.roots {
            root.expanded = false;
        }
        tree.rebuild_visible();
        assert_eq!(tree.len(), 1);

        // Reveal the deep track: should expand A, B, C and return its index
        let idx = tree.reveal_track(TrackId(1)).expect("track should be revealed");
        let item = &tree.visible_items[idx];
        assert_eq!(item.track_id, Some(TrackId(1)));
        assert_eq!(item.name, "Deep Song");
    }

    #[test]
    fn test_tree_collect_track_ids() {
        let mut lib = Library::new();
        lib.insert(create_test_track(1, "/music/Rock/01.mp3", "Track 1"));
        lib.insert(create_test_track(2, "/music/Rock/02.mp3", "Track 2"));

        let mut tree = LibraryTree::new();
        tree.build(&lib, &[PathBuf::from("/music")]);

        let rock_idx = tree.visible_items.iter().position(|i| i.name == "Rock").unwrap();
        let ids = tree.collect_track_ids(rock_idx);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&TrackId(1)));
        assert!(ids.contains(&TrackId(2)));
    }
}
