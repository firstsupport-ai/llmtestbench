use postman2openapi::{ TranspileOptions, TargetFormat };

fn main() {
    println!("cargo::rerun-if-changed=postman_collection.json");
    
    std::fs::write(
        "openapi_collection.json",
        postman2openapi::from_path(
            "postman_collection.json",
            TranspileOptions {
                format: TargetFormat::Json
            }
        ).unwrap()
    ).unwrap();
}
