// use std::collections::HashMap;
// use crate::Schema;
// 
// #[derive(Debug)]
// pub struct SchemaManager {
//     schemas: HashMap<String, Schema>
// }
// 
// impl SchemaManager {
//     pub fn parse_from_directory(dir: &str) -> Result<Self, String> {
//         use std::fs;
//         use std::collections::HashMap;
//         use crate::Schema;
// 
//         let mut schemas = HashMap::new();
// 
//         for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))? {
//             let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
//             let path = entry.path();
//             let path_str = path.to_str().ok_or("Invalid path")?;
// 
//             if path.is_dir() {
//                 let sub_schemas = Self::parse_from_directory(path_str)?;
//                 schemas.extend(sub_schemas.schemas);
//             } else if path.extension().and_then(|s| s.to_str()) == Some("quops") {
//                 let schema = Schema::parse_from_file(path_str.into())?;
//                 if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
//                     schemas.insert(name.to_string(), schema);
//                 }
//             }
//         }
// 
//         Ok(SchemaManager { schemas })
//     }
// 
//     pub fn get_schema(&self, name: &str) -> Option<&Schema> {
//         self.schemas.get(name)
//     }
// }