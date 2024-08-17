use super::*;

pub enum BackgroundTaskResult {
    ProfileCreated(Result<(), ApplicationError>),
}
pub struct NewProfileCreator {
    pub predefined_types: Vec<String>,
    pub selected_type: usize,
    pub selected_model_index: usize, // New field for model selection
    pub background_task: Option<mpsc::Receiver<BackgroundTaskResult>>,
    pub task_start_time: Option<Instant>,
    pub spinner_state: usize,
    pub new_profile_name: Option<String>,
    pub available_models: Vec<ModelSpec>,
    pub model_selection_pending: bool,
    pub db_handler: UserProfileDbHandler,
}

impl NewProfileCreator {
    pub fn new(db_handler: UserProfileDbHandler) -> Self {
        Self {
            predefined_types: SUPPORTED_MODEL_ENDPOINTS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            selected_type: 0,
            selected_model_index: 0, // Initialize the new field
            background_task: None,
            task_start_time: None,
            spinner_state: 0,
            new_profile_name: None,
            available_models: Vec::new(),
            model_selection_pending: false,
            db_handler,
        }
    }

    pub async fn prepare_for_model_selection(
        &mut self,
    ) -> Result<bool, ApplicationError> {
        let profile_type = &self.predefined_types[self.selected_type];
        let model_server = ModelServer::from_str(profile_type)?;

        match model_server.list_models().await {
            Ok(models) if !models.is_empty() => {
                self.available_models = models;
                self.model_selection_pending = true;
                self.selected_model_index = 0; // Reset model selection index
                Ok(true)
            }
            Ok(_) => {
                println!("No models available for this server.");
                self.model_selection_pending = false;
                Ok(false)
            }
            Err(ApplicationError::NotReady(msg)) => {
                println!(
                    "Server not ready: {}. Model selection will be skipped.",
                    msg
                );
                self.model_selection_pending = false;
                Ok(false)
            }
            Err(e) => Err(e),
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

        let model_server = ModelServer::from_str(profile_type)?;
        let server_settings = model_server.get_profile_settings();
        if let JsonValue::Object(map) = server_settings {
            for (key, value) in map {
                settings.insert(key, value);
            }
        }

        if self.model_selection_pending {
            if let Some(selected_model) =
                self.available_models.get(self.selected_model_index)
            {
                settings.insert(
                    "__MODEL_IDENTIFIER".to_string(),
                    json!(selected_model.identifier.0),
                );
            }
        }

        let mut db_handler = db_handler.clone();
        let (tx, rx) = mpsc::channel(1);
        let new_profile_name_clone = new_profile_name.clone();
        let settings_clone = settings.clone();
        tokio::spawn(async move {
            let result = db_handler
                .create_or_update(
                    &new_profile_name_clone,
                    &json!(settings_clone),
                )
                .await;
            let _ = tx.send(BackgroundTaskResult::ProfileCreated(result)).await;
        });
        self.background_task = Some(rx);
        self.task_start_time = Some(Instant::now());
        self.spinner_state = 0;
        self.new_profile_name = Some(new_profile_name);
        Ok(())
    }

    // Add methods to manipulate selected_model_index
    pub fn move_model_selection_up(&mut self) {
        if self.selected_model_index > 0 {
            self.selected_model_index -= 1;
        }
    }

    pub fn move_model_selection_down(&mut self) {
        if self.selected_model_index < self.available_models.len() - 1 {
            self.selected_model_index += 1;
        }
    }
}
