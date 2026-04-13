use csv::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

pub struct CsvDataSource {
    records: Vec<HashMap<String, String>>,
}

impl CsvDataSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mut rdr = Reader::from_reader(file);
        let headers = rdr.headers().map_err(|e| e.to_string())?.clone();

        let mut records = Vec::new();
        for result in rdr.records() {
            let record = result.map_err(|e| e.to_string())?;
            let mut row = HashMap::new();
            for (i, header) in headers.iter().enumerate() {
                row.insert(header.to_string(), record.get(i).unwrap_or("").to_string());
            }
            records.push(row);
        }

        Ok(CsvDataSource { records })
    }

    pub fn get_record(&self, index: usize) -> Option<&HashMap<String, String>> {
        self.records.get(index % self.records.len())
    }
}
