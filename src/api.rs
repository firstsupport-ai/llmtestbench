use actix_web::web::{self, ServiceConfig};

mod analyze;

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(web::scope("/analyze")
            .configure(analyze::attach));
}
