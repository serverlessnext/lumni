use super::*;

pub enum BackgroundTaskResult {
    ProfileCreated(Result<(), ApplicationError>),
}

pub struct NewProfileCreator {
    pub predefined_types: Vec<String>,
    pub selected_type: usize,
    pub background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    pub task_start_time: Option<Instant>,
    pub spinner_state: usize,
    pub new_profile_name: Option<String>,
}

impl NewProfileCreator {
    pub fn new() -> Self {
        Self {
            predefined_types: vec![
                "Custom".to_string(),
                "OpenAI".to_string(),
                "Anthropic".to_string(),
            ],
            selected_type: 0,
            background_task: None,
            task_start_time: None,
            spinner_state: 0,
            new_profile_name: None,
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_type > 0 {
            self.selected_type -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected_type < self.predefined_types.len() - 1 {
            self.selected_type += 1;
        }
    }

    pub async fn create_new_profile(
        &mut self,
        db_handler: &UserProfileDbHandler,
        profile_count: usize,
    ) -> Result<(), ApplicationError> {
        let new_profile_name = format!("New_Profile_{}", profile_count + 1);
        let profile_type = &self.predefined_types[self.selected_type];

        let mut settings = Map::new();
        settings.insert("__PROFILE_TYPE".to_string(), json!(profile_type));

        // Add default settings based on the profile type
        match profile_type.as_str() {
            "OpenAI" => {
                settings.insert("api_key".to_string(), json!(""));
                settings.insert("model".to_string(), json!("gpt-3.5-turbo"));
            }
            "Anthropic" => {
                settings.insert("api_key".to_string(), json!(""));
                settings.insert("model".to_string(), json!("claude-2"));
            }
            "Custom" => {}
            _ => {
                return Err(ApplicationError::InvalidInput(
                    "Unknown profile type".to_string(),
                ))
            }
        }

        let mut db_handler = db_handler.clone();
        let (tx, rx) = mpsc::channel(1);

        let new_profile_name_clone = new_profile_name.clone();
        tokio::spawn(async move {
            let result = db_handler
                .create_or_update(&new_profile_name_clone, &json!(settings))
                .await;
            let _ = tx.send(BackgroundTaskResult::ProfileCreated(result)).await;
        });

        self.background_task = Some(rx);
        self.task_start_time = Some(Instant::now());
        self.spinner_state = 0;
        self.new_profile_name = Some(new_profile_name);

        Ok(())
    }

    pub fn get_predefined_types(&self) -> &[String] {
        &self.predefined_types
    }

    pub fn get_selected_type(&self) -> usize {
        self.selected_type
    }

    pub fn get_background_task(
        &mut self,
    ) -> Option<&mut mpsc::Receiver<BackgroundTaskResult>> {
        self.background_task.as_mut()
    }

    pub fn get_task_start_time(&self) -> Option<Instant> {
        self.task_start_time
    }

    pub fn get_spinner_state(&self) -> usize {
        self.spinner_state
    }

    pub fn increment_spinner_state(&mut self) {
        self.spinner_state = (self.spinner_state + 1) % 10; // Assuming 10 spinner states
    }

    pub fn take_new_profile_name(&mut self) -> Option<String> {
        self.new_profile_name.take()
    }

    pub fn clear_background_task(&mut self) {
        self.background_task = None;
        self.task_start_time = None;
    }
}
