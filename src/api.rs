use actix_web::web::{self, ServiceConfig};

mod analyze;
mod auth;
mod post_analyze;
mod external;

pub(super) fn attach(app: &mut ServiceConfig) {
    app
        .service(web::scope("/")
            .configure(auth::attach))
        .service(web::scope("/analyze")
            .configure(analyze::attach))
        .service(web::scope("/post_analyze")
            .configure(post_analyze::attach))
        .service(web::scope("/external")
            .configure(external::attach));
}
