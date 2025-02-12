use actix_web::web::{self, ServiceConfig};

mod model;
mod analyze;

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(web::scope("/model")
            .configure(model::attach))
        .service(web::scope("/analyze")
            .configure(analyze::attach));
}
