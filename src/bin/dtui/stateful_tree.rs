use tui_tree_widget::{TreeItem, TreeState};

pub struct StatefulTree<'a> {
    pub state: TreeState<usize>,
    pub items: Vec<TreeItem<'a, usize>>,
}

impl<'a> Default for StatefulTree<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> StatefulTree<'a> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            state: TreeState::default(),
            items: Vec::new(),
        }
    }

    pub fn with_items(items: Vec<TreeItem<'a, usize>>) -> Self {
        Self {
            state: TreeState::default(),
            items,
        }
    }

    pub fn down(&mut self) {
        self.state.key_down(&self.items);
    }

    pub fn up(&mut self) {
        self.state.key_up(&self.items);
    }

    pub fn left(&mut self) {
        self.state.key_left();
    }

    pub fn right(&mut self) {
        self.state.key_right();
    }

    pub fn toggle(&mut self) {
        self.state.toggle_selected();
    }
}
