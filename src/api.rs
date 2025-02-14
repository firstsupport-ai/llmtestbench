use actix_web::web::{self, ServiceConfig};

mod analyze;
mod auth;

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(web::scope("/analyze")
            .configure(analyze::attach))
        .service(web::scope("/auth")
            .configure(auth::attach));
}
