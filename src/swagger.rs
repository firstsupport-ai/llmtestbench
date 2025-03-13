use actix_web::{get, web::ServiceConfig, HttpRequest, HttpResponse, Responder};

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(serve)
        .service(serve_json);
}

#[get("")]
async fn serve(req: HttpRequest) -> impl Responder {
    const TEMPLATE: &str = include_str!("../template/swagger.liquid");

    let template = liquid::ParserBuilder::with_stdlib().build().unwrap()
        .parse(TEMPLATE).unwrap();

    HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render(&liquid::object!({
            "swagger_uri": format!("{}/swagger.json", req.full_url().as_str())
        })).unwrap())
}

#[get("/swagger.json")]
async fn serve_json() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(include_str!("../openapi_collection.json"))
}
