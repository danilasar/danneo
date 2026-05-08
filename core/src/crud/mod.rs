use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};
use sea_query::{
    Alias, ColumnDef, Keyword, MysqlQueryBuilder, PostgresQueryBuilder, Query, SimpleExpr,
    SqliteQueryBuilder, Table, TableCreateStatement,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema describing a dynamic entity table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchema {
    pub table_name: String,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: String,
    pub primary_key: Option<bool>,
    pub auto_increment: Option<bool>,
    pub nullable: Option<bool>,
    pub unique: Option<bool>,
    pub default: Option<Value>,
    pub label: Option<String>,
}

fn row_value(row: &sea_orm::QueryResult, col: &str) -> Value {
    if let Ok(Some(value)) = row.try_get::<Option<String>>("", col) {
        return Value::String(value);
    }
    if let Ok(Some(value)) = row.try_get::<Option<bool>>("", col) {
        return Value::Bool(value);
    }
    if let Ok(Some(value)) = row.try_get::<Option<i64>>("", col) {
        return Value::Number(value.into());
    }
    if let Ok(Some(value)) = row.try_get::<Option<i32>>("", col) {
        return Value::Number(value.into());
    }
    if let Ok(Some(value)) = row.try_get::<Option<f64>>("", col) {
        if let Some(number) = serde_json::Number::from_f64(value) {
            return Value::Number(number);
        }
    }
    Value::Null
}

fn build_create_table(schema: &EntitySchema) -> TableCreateStatement {
    let mut table = Table::create();
    table.table(Alias::new(&schema.table_name)).if_not_exists();

    for field in &schema.fields {
        let mut col = ColumnDef::new(Alias::new(&field.name));
        match field.field_type.as_str() {
            "integer" => {
                col.integer();
            }
            "bigint" => {
                col.big_integer();
            }
            "string" => {
                col.string_len(255);
            }
            "text" => {
                col.text();
            }
            "boolean" => {
                col.boolean();
            }
            "datetime" => {
                col.timestamp();
            }
            other => {
                col.text();
                tracing::warn!("Unknown field type '{}', defaulting to TEXT", other);
            }
        }
        if field.primary_key.unwrap_or(false) {
            col.primary_key();
        }
        if field.auto_increment.unwrap_or(false) {
            col.auto_increment();
        }
        if field.unique.unwrap_or(false) {
            col.unique_key();
        }

        // Fix: Ensure we don't send both null and not_null or conflict with PK defaults
        if field.primary_key.unwrap_or(false) {
            // PK is usually NOT NULL by default in most DBs, but let's be explicit
            col.not_null();
        } else if field.nullable.unwrap_or(true) {
            col.null();
        } else {
            col.not_null();
        }
        if let Some(def) = &field.default {
            if let Some(s) = def.as_str() {
                if s.to_uppercase() == "CURRENT_TIMESTAMP" {
                    col.default(Keyword::CurrentTimestamp);
                } else {
                    col.default(s);
                }
            } else if let Some(i) = def.as_i64() {
                col.default(i);
            } else if let Some(b) = def.as_bool() {
                col.default(b);
            }
        }
        table.col(&mut col);
    }
    table.to_owned()
}

/// Create a table from `EntitySchema` using sea_query. Works with any backend.
pub async fn create_entity_table(
    db: &DatabaseConnection,
    schema: &EntitySchema,
) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let stmt = match backend {
        sea_orm::DatabaseBackend::Postgres => {
            build_create_table(schema).build(PostgresQueryBuilder)
        }
        sea_orm::DatabaseBackend::MySql => build_create_table(schema).build(MysqlQueryBuilder),
        sea_orm::DatabaseBackend::Sqlite => build_create_table(schema).build(SqliteQueryBuilder),
    };
    db.execute(Statement::from_string(backend, stmt)).await?;
    Ok(())
}

/// Drop a dynamic entity table.
pub async fn drop_entity_table(db: &DatabaseConnection, table_name: &str) -> Result<(), DbErr> {
    let _backend = db.get_database_backend();
    let stmt = match db.get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => Table::drop()
            .table(Alias::new(table_name))
            .if_exists()
            .build(PostgresQueryBuilder),
        sea_orm::DatabaseBackend::MySql => Table::drop()
            .table(Alias::new(table_name))
            .if_exists()
            .build(MysqlQueryBuilder),
        sea_orm::DatabaseBackend::Sqlite => Table::drop()
            .table(Alias::new(table_name))
            .if_exists()
            .build(SqliteQueryBuilder),
    };
    db.execute(Statement::from_string(db.get_database_backend(), stmt))
        .await?;
    Ok(())
}

/// Insert a record; returns the data back as confirmation.
pub async fn insert_record(
    db: &DatabaseConnection,
    table: &str,
    data: &Value,
) -> Result<Value, DbErr> {
    let backend = db.get_database_backend();
    let mut query = Query::insert();
    query.into_table(Alias::new(table));
    if let Some(obj) = data.as_object() {
        let mut cols: Vec<Alias> = Vec::new();
        let mut vals: Vec<SimpleExpr> = Vec::new();
        for (k, v) in obj {
            cols.push(Alias::new(k));
            let expr = match v {
                Value::String(s) => SimpleExpr::Value(s.clone().into()),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        SimpleExpr::Value(i.into())
                    } else {
                        SimpleExpr::Value(n.as_f64().unwrap().into())
                    }
                }
                Value::Bool(b) => SimpleExpr::Value((*b).into()),
                Value::Null => SimpleExpr::Value(sea_orm::Value::Int(None).into()),
                _ => SimpleExpr::Value(v.to_string().into()),
            };
            vals.push(expr);
        }
        query.columns(cols).values(vals).unwrap();
    }
    let (sql, values) = match backend {
        sea_orm::DatabaseBackend::Postgres => query.build(PostgresQueryBuilder),
        sea_orm::DatabaseBackend::MySql => query.build(MysqlQueryBuilder),
        sea_orm::DatabaseBackend::Sqlite => query.build(SqliteQueryBuilder),
    };
    db.execute(Statement::from_sql_and_values(backend, &sql, values))
        .await?;
    Ok(data.clone())
}

/// Return all rows from a dynamic table as JSON objects.
pub async fn select_all(
    db: &DatabaseConnection,
    table: &str,
    columns: &[String],
) -> Result<Vec<Value>, DbErr> {
    let backend = db.get_database_backend();
    let mut query = Query::select();
    query.from(Alias::new(table)).column(sea_query::Asterisk);
    let (sql, values) = match backend {
        sea_orm::DatabaseBackend::Postgres => query.build(PostgresQueryBuilder),
        sea_orm::DatabaseBackend::MySql => query.build(MysqlQueryBuilder),
        sea_orm::DatabaseBackend::Sqlite => query.build(SqliteQueryBuilder),
    };
    let rows = db
        .query_all(Statement::from_sql_and_values(backend, &sql, values))
        .await?;

    let mut results = Vec::new();
    for row in rows {
        let mut map = serde_json::Map::new();
        for col in columns {
            map.insert(col.clone(), row_value(&row, col));
        }
        results.push(Value::Object(map));
    }
    Ok(results)
}

/// Select a single record by primary key.
pub async fn select_by_pk(
    db: &DatabaseConnection,
    table: &str,
    columns: &[String],
    pk_col: &str,
    pk_val: &str,
) -> Result<Option<Value>, DbErr> {
    let backend = db.get_database_backend();
    let mut query = Query::select();
    query
        .from(Alias::new(table))
        .column(sea_query::Asterisk)
        .and_where(sea_query::Expr::col(Alias::new(pk_col)).eq(pk_val));

    let (sql, values) = match backend {
        sea_orm::DatabaseBackend::Postgres => query.build(PostgresQueryBuilder),
        sea_orm::DatabaseBackend::MySql => query.build(MysqlQueryBuilder),
        sea_orm::DatabaseBackend::Sqlite => query.build(SqliteQueryBuilder),
    };
    let row = db
        .query_one(Statement::from_sql_and_values(backend, &sql, values))
        .await?;

    if let Some(row) = row {
        let mut map = serde_json::Map::new();
        for col in columns {
            map.insert(col.clone(), row_value(&row, col));
        }
        Ok(Some(Value::Object(map)))
    } else {
        Ok(None)
    }
}

/// Update a record by primary key.
pub async fn update_record(
    db: &DatabaseConnection,
    table: &str,
    pk_col: &str,
    pk_val: &str,
    data: &Value,
) -> Result<Value, DbErr> {
    let backend = db.get_database_backend();
    let mut query = Query::update();
    query
        .table(Alias::new(table))
        .and_where(sea_query::Expr::col(Alias::new(pk_col)).eq(pk_val));

    if let Some(obj) = data.as_object() {
        for (k, v) in obj {
            if k == pk_col {
                continue;
            } // Don't update PK
            let expr = match v {
                Value::String(s) => SimpleExpr::Value(s.clone().into()),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        SimpleExpr::Value(i.into())
                    } else {
                        SimpleExpr::Value(n.as_f64().unwrap().into())
                    }
                }
                Value::Bool(b) => SimpleExpr::Value((*b).into()),
                Value::Null => SimpleExpr::Value(sea_orm::Value::Int(None).into()),
                _ => SimpleExpr::Value(v.to_string().into()),
            };
            query.value(Alias::new(k), expr);
        }
    }

    let (sql, values) = match backend {
        sea_orm::DatabaseBackend::Postgres => query.build(PostgresQueryBuilder),
        sea_orm::DatabaseBackend::MySql => query.build(MysqlQueryBuilder),
        sea_orm::DatabaseBackend::Sqlite => query.build(SqliteQueryBuilder),
    };
    db.execute(Statement::from_sql_and_values(backend, &sql, values))
        .await?;
    Ok(data.clone())
}

/// Delete a record by primary key.
pub async fn delete_record(
    db: &DatabaseConnection,
    table: &str,
    pk_col: &str,
    pk_val: &str,
) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let mut query = Query::delete();
    query
        .from_table(Alias::new(table))
        .and_where(sea_query::Expr::col(Alias::new(pk_col)).eq(pk_val));

    let (sql, values) = match backend {
        sea_orm::DatabaseBackend::Postgres => query.build(PostgresQueryBuilder),
        sea_orm::DatabaseBackend::MySql => query.build(MysqlQueryBuilder),
        sea_orm::DatabaseBackend::Sqlite => query.build(SqliteQueryBuilder),
    };
    db.execute(Statement::from_sql_and_values(backend, &sql, values))
        .await?;
    Ok(())
}

// ── helpers ─────────────────────────────────────────────────────────────────

// Helper not needed after simplifying builder usage
