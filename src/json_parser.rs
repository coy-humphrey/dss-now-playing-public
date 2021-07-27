use serde_json::Value;

// Ideally, the parser would be able to parse the json into a strongly-typed
// rust data structure that would cover all the different cases. Even better if
// the definition for the data structure came from a library shared by both the client
// and the server implementation.
// As it is, this parser was optimized for fast development time. It handles the bare minimum
// of cases required to get some images (of 1 specific aspect ratio) onto the screen.
pub struct JsonParser {
    main: Value,
}

#[derive(Debug)]
pub struct TileInfo {
    pub title: String,
    pub img_url: String,
}

#[derive(Debug)]
pub struct RowInfo {
    pub title: String,
    pub tiles: Vec<TileInfo>,
}

const MAIN_URL: &str = "https://cd-static.bamgrid.com/dp-117731241344/home.json";

impl JsonParser {
    pub fn new() -> Self {
        JsonParser { main: Value::Null }
    }

    fn parse_main_json(&mut self) {
        self.main = reqwest::blocking::get(MAIN_URL).unwrap().json().unwrap();
    }

    fn parse_ref_json(ref_id: &str) -> Value {
        let url = format!(
            "https://cd-static.bamgrid.com/dp-117731241344/sets/{}.json",
            ref_id
        );
        reqwest::blocking::get(url).unwrap().json().unwrap()
    }

    pub fn parse_all_rows(&mut self) -> Vec<RowInfo> {
        self.parse_main_json();
        let mut rows = Vec::new();

        let mut i = 0;
        while !matches!(self.get_container(i), Value::Null) {
            let mut container = self.get_container(i);
            let mut row_info = RowInfo {
                title: Self::get_container_title(container),
                tiles: Vec::new(),
            };

            let ref_container;
            if let Value::String(ref_id) = &container["refId"] {
                ref_container = Self::parse_ref_json(ref_id);
                for val in &["CuratedSet", "TrendingSet", "PersonalizedCuratedSet"] {
                    if !matches!(&ref_container["data"][val], Value::Null) {
                        container = &ref_container["data"][val];
                    }
                }
            }

            let mut j = 0;
            while !matches!(Self::get_container_item(container, j), Value::Null) {
                let item = Self::get_container_item(container, j);
                let tile_info = TileInfo {
                    title: Self::get_item_title(item).unwrap_or_else(|| "".to_string()),
                    img_url: Self::get_item_image_url(item).unwrap_or_else(|| "".to_string()),
                };
                row_info.tiles.push(tile_info);
                j += 1;
            }
            if !row_info.tiles.is_empty() {
                rows.push(row_info);
            } else {
                println!("Could not parse tiles from row {}", i);
            }
            i += 1;
        }

        rows
    }

    pub fn get_container(&self, idx: usize) -> &Value {
        &self.main["data"]["StandardCollection"]["containers"][idx]["set"]
    }

    pub fn get_container_title(container: &Value) -> String {
        if let Value::String(string) =
            &container["text"]["title"]["full"]["set"]["default"]["content"]
        {
            string.clone()
        } else {
            "".to_string()
        }
    }

    pub fn get_container_item(container: &Value, idx: usize) -> &Value {
        &container["items"][idx]
    }

    pub fn get_item_title(item: &Value) -> Option<String> {
        let prefix = &item["text"]["title"]["full"];
        let mut variable = &Value::Null;
        // May be more possible options here
        for val in &["series", "program", "collection"] {
            match prefix[val] {
                Value::Null => continue,
                _ => variable = &prefix[val],
            }
        }
        if let Value::String(string) = &variable["default"]["content"] {
            Some(string.clone())
        } else {
            None
        }
    }

    pub fn get_item_image_url(item: &Value) -> Option<String> {
        let prefix = &item["image"]["tile"]["1.78"];
        let mut variable = &Value::Null;
        for val in &["series", "program", "collection", "default"] {
            match prefix[val] {
                Value::Null => continue,
                _ => variable = &prefix[val],
            }
        }
        if let Value::String(string) = &variable["default"]["url"] {
            Some(string.clone())
        } else {
            None
        }
    }
}

// These tests are extremely fragile and based off of specific values in the json
// on the date the tests were written. They were used to quickly confirm the parser's
// correct behavior during development. They're left commented out here for historical perspective.

// #[cfg(test)]
// mod test {
//     use super::*;

//     // Test main parser can run without crashing
//     #[test]
//     fn test_parse() {
//         let mut parser = JsonParser::new();
//         parser.parse_main_json();
//     }

//     #[test]
//     fn test_container_parsing() {
//         let mut parser = JsonParser::new();
//         parser.parse_main_json();
//         assert!(matches!(parser.get_container(0), Value::Object { .. }));
//         assert!(matches!(parser.get_container(10000000), Value::Null));
//     }

//     #[test]
//     fn test_title_parsing() {
//         let mut parser = JsonParser::new();
//         parser.parse_main_json();
//         assert!(matches!(parser.get_container(0), Value::Object { .. }));
//         let container = parser.get_container(0);
//         let title = JsonParser::get_container_title(container);
//         assert_eq!(title, "New to Disney+");
//     }

//     #[test]
//     fn test_item_parsing() {
//         let mut parser = JsonParser::new();
//         parser.parse_main_json();
//         let container = parser.get_container(0);
//         let item = JsonParser::get_container_item(container, 0);
//         let title = JsonParser::get_item_title(item).unwrap();
//         assert_eq!(title, "The Right Stuff");
//         let url = JsonParser::get_item_image_url(item).unwrap();
//         assert_eq!(url, "https://prod-ripcut-delivery.disney-plus.net/v1/variant/disney/3C33485A3043C22B8C89E131693E8B5B9306DAA4E48612A655560752977728A6/scale?format=jpeg&quality=90&scalingAlgorithm=lanczos3&width=500");
//     }

//     #[test]
//     fn test_container_iter() {
//         let mut parser = JsonParser::new();
//         parser.parse_main_json();
//         let mut i = 0;
//         while !matches!(parser.get_container(i), Value::Null) {
//             i += 1;
//         }
//         // There are 13 upper level containers in sample dataset
//         assert_eq!(i, 13);
//     }

//     #[test]
//     fn test_container_item_iter() {
//         let mut parser = JsonParser::new();
//         parser.parse_main_json();
//         let container = parser.get_container(0);
//         let mut i = 0;
//         while !matches!(JsonParser::get_container_item(container, i), Value::Null) {
//             i += 1;
//         }
//         assert_eq!(i, 15);
//     }

//     #[test]
//     fn test_parse_tiles() {
//         let mut parser = JsonParser::new();
//         parser.parse_main_json();
//         let tile_rows = parser.parse_all_rows();
//         println!("{:?}", tile_rows);
//         println!("Rows: {}", tile_rows.len());
//     }
// }
