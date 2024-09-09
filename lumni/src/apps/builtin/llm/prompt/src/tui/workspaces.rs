use std::path::PathBuf;

use super::conversations::Conversations;
use super::WorkspaceId;

pub struct Workspaces {
    pub workspaces: Vec<Workspace>,
    pub current_workspace_index: usize,
}

pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub directory_path: Option<PathBuf>,
    pub conversations: Conversations,
}

impl Workspaces {
    pub fn new_as_default(conversations: Conversations) -> Self {
        let default_workspace = Workspace {
            id: WorkspaceId(1),
            name: "Default".to_string(),
            directory_path: None,
            conversations,
        };

        Self {
            workspaces: vec![default_workspace],
            current_workspace_index: 0,
        }
    }

    pub fn current_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(self.current_workspace_index)
    }

    pub fn current_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.current_workspace_index)
    }

    pub fn current_conversations_mut(&mut self) -> Option<&mut Conversations> {
        self.current_workspace_mut().map(|w| &mut w.conversations)
    }
}
