#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        extract::Request as AxumRequest,
        http::{Request, StatusCode},
        middleware::Next,
        routing::get,
    };
    use danneo_core::module::DanneoModule;
    use danneo_core::state::AppState;
    use std::sync::Arc;
    use tower::ServiceExt;

    struct MiddlewareModule;
    #[async_trait]
    impl DanneoModule for MiddlewareModule {
        fn name(&self) -> &'static str {
            "test_middleware"
        }

        fn register_admin_middleware(
            &self,
            router: Router<Arc<AppState>>,
            state: Arc<AppState>,
        ) -> Router<Arc<AppState>> {
            router.layer(axum::middleware::from_fn_with_state(
                state,
                |req: AxumRequest, next: Next| async move {
                    let mut res = next.run(req).await;
                    res.headers_mut()
                        .insert("X-Test-Middleware", "Applied".parse().unwrap());
                    res
                },
            ))
        }
    }

    #[tokio::test]
    async fn test_modular_middleware() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // AppState::new runs migrations, so we need a real DB or mock
        // For simplicity in this test, we can try to mock or just use sqlite memory
        let state = danneo_core::state::init_state(db).await.unwrap();

        let mut router = Router::new().route("/test", get(|| async { "ok" }));

        // Apply module's middleware to the router BEFORE with_state if we want S to be Arc<AppState>
        let module = MiddlewareModule;
        router = module.register_admin_middleware(router, state.clone());

        let app = router.with_state(state.clone());

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("X-Test-Middleware").unwrap(),
            "Applied"
        );
    }
}
