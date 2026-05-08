use casbin::function_map::OperatorFunction;
use casbin::{CoreApi, DefaultModel, Enforcer, FileAdapter, MgmtApi, RbacApi};
// Используем Dynamic из casbin, так как он сконфигурирован с only_i32 и без функций
use casbin::rhai::Dynamic;
use sea_orm::DatabaseConnection;
use sea_orm_adapter::SeaOrmAdapter;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AclService {
    enforcer: Arc<RwLock<Enforcer>>,
}

/// Функция для сравнения уровней доступа в матчере Casbin
fn match_level(a: Dynamic, b: Dynamic) -> Dynamic {
    let r_level = a.as_int().unwrap_or(0);
    let p_level = if b.is_int() {
        b.as_int().unwrap()
    } else {
        b.clone()
            .into_string()
            .unwrap_or_default()
            .parse::<i32>()
            .unwrap_or(0)
    };
    (r_level >= p_level).into()
}

impl AclService {
    pub fn enforcer(&self) -> Arc<RwLock<Enforcer>> {
        self.enforcer.clone()
    }

    /// Создает новый сервис на основе базы данных
    pub async fn new_db(db: Arc<DatabaseConnection>, model_path: &str) -> Self {
        let m = DefaultModel::from_file(model_path)
            .await
            .expect("Failed to load Casbin model");
        let a = SeaOrmAdapter::new((*db).clone())
            .await
            .expect("Failed to initialize SeaORM adapter");
        let mut e = Enforcer::new(m, a)
            .await
            .expect("Failed to create Enforcer");

        e.add_function("matchLevel", OperatorFunction::Arg2(match_level));

        Self {
            enforcer: Arc::new(RwLock::new(e)),
        }
    }

    /// Создает новый сервис на основе файлов (для тестов)
    pub async fn new_file(model_path: String, policy_path: String) -> Self {
        let m = DefaultModel::from_file(&model_path).await.unwrap();
        let a = FileAdapter::new(policy_path);
        let mut e = Enforcer::new(m, a).await.unwrap();

        e.add_function("matchLevel", OperatorFunction::Arg2(match_level));

        Self {
            enforcer: Arc::new(RwLock::new(e)),
        }
    }

    pub async fn enforce(&self, sub: &str, obj: &str, act: &str, level: i32) -> bool {
        let e = self.enforcer.read().await;
        e.enforce((sub, obj, act, level)).unwrap_or(false)
    }

    /// Добавление роли пользователю
    pub async fn add_role_for_user(&self, user: &str, role: &str) -> bool {
        let mut e = self.enforcer.write().await;
        e.add_role_for_user(user, role, None).await.unwrap_or(false)
    }

    /// Удаление роли у пользователя
    pub async fn delete_role_for_user(&self, user: &str, role: &str) -> bool {
        let mut e = self.enforcer.write().await;
        e.delete_role_for_user(user, role, None)
            .await
            .unwrap_or(false)
    }

    /// Добавление политики (права доступа)
    pub async fn add_policy(&self, sub: &str, obj: &str, act: &str, level: i32) -> bool {
        let mut e = self.enforcer.write().await;
        e.add_policy(vec![
            sub.to_string(),
            obj.to_string(),
            act.to_string(),
            level.to_string(),
        ])
        .await
        .unwrap_or(false)
    }

    /// Удаление всех политик для субъекта (роли)
    pub async fn remove_filtered_policy(&self, field_index: usize, field_value: &str) -> bool {
        let mut e = self.enforcer.write().await;
        e.remove_filtered_policy(field_index, vec![field_value.to_string()])
            .await
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_rbac_and_mac() {
        let mut model_file = NamedTempFile::new().unwrap();
        writeln!(model_file, r#"
[request_definition]
r = sub, obj, act, level
[policy_definition]
p = sub, obj, act, level
[role_definition]
g = _, _
[policy_effect]
e = some(where (p.eft == allow))
[matchers]
m = g(r.sub, p.sub) && (r.obj == p.obj || p.obj == "*") && (r.act == p.act || p.act == "*") && matchLevel(r.level, p.level)
        "#).unwrap();

        let mut policy_file = NamedTempFile::new().unwrap();
        writeln!(
            policy_file,
            r#"
p, role:admin, settings, edit, 80
p, role:admin, news, edit, 10
p, role:manager, news, edit, 10
p, role:super, *, *, 100
g, alice, role:admin
g, bob, role:manager
g, root, role:super
        "#
        )
        .unwrap();

        let acl = AclService::new_file(
            model_file.path().to_str().unwrap().to_string(),
            policy_file.path().to_str().unwrap().to_string(),
        )
        .await;

        assert!(acl.enforce("alice", "settings", "edit", 90).await);
        assert!(!acl.enforce("alice", "settings", "edit", 50).await);
        assert!(acl.enforce("bob", "news", "edit", 20).await);
        assert!(!acl.enforce("bob", "settings", "edit", 20).await);
        assert!(acl.enforce("root", "any", "any", 100).await);
        assert!(!acl.enforce("root", "any", "any", 50).await);
    }
}
