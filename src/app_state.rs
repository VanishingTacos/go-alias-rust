use std::collections::HashMap;
use std::sync::Mutex;

use crate::sql::DbConnection;

pub struct AppState {
    pub shortcuts: HashMap<String, String>,
    pub notes: Mutex<Vec<String>>,

    // SQL service state
    pub connections: Mutex<Vec<DbConnection>>,
    pub last_results: Mutex<Vec<HashMap<String, String>>>,
}
