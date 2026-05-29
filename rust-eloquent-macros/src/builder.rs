use quote::quote;
use proc_macro2::TokenStream;
use crate::parser::ParsedModel;

/// Generates the magic methods for each field (where_field, order_by_field, etc)
fn generate_magic_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
    let mut magic_methods = vec![];
    for field_name in &parsed.normal_fields {
        let field_name_str = field_name.to_string();
        
        let where_method = quote::format_ident!("where_{}", field_name);
        let or_where_method = quote::format_ident!("or_where_{}", field_name);
        let where_not_method = quote::format_ident!("where_not_{}", field_name);
        
        magic_methods.push(quote! {
            pub fn #where_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.where_eq(#field_name_str, value)
            }
            pub fn #or_where_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.or_where(#field_name_str, value)
            }
            pub fn #where_not_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.where_not_eq(#field_name_str, value)
            }
        });

        let order_by_method = quote::format_ident!("order_by_{}", field_name);
        let order_by_desc_method = quote::format_ident!("order_by_{}_desc", field_name);
        magic_methods.push(quote! {
            pub fn #order_by_method(self) -> Self {
                self.order_by(#field_name_str)
            }
            pub fn #order_by_desc_method(self) -> Self {
                self.order_by_desc(#field_name_str)
            }
        });
    }
    magic_methods
}

/// Generates the delete_all logic based on soft deletes
fn generate_delete_all_logic(has_soft_deletes: bool, table_name: &str) -> TokenStream {
    if has_soft_deletes {
        quote! {
            let mut query_str = format!("UPDATE {} SET deleted_at = CURRENT_TIMESTAMP", #table_name);
        }
    } else {
        quote! {
            let mut query_str = format!("DELETE FROM {}", #table_name);
        }
    }
}

pub fn generate(
    parsed: &ParsedModel,
    relation_flags: &[TokenStream],
    relation_inits: &[TokenStream],
    relation_methods: &[TokenStream],
    eager_loads: &TokenStream,
) -> TokenStream {
    let name = &parsed.name;
    let column_enum_name = quote::format_ident!("{}Column", name);
    let builder_name = quote::format_ident!("{}QueryBuilder", name);
    let table_name = &parsed.table_name;
    let has_soft_deletes = parsed.has_soft_deletes;
    let hook_after_fetch = if !parsed.after_fetch.is_empty() {
        let method = syn::Ident::new(&parsed.after_fetch, name.span());
        quote! { for model in &mut results { model.#method().await?; } }
    } else {
        quote! {}
    };

    let delete_all_logic = generate_delete_all_logic(has_soft_deletes, table_name);
    let magic_methods = generate_magic_methods(parsed);

    quote! {
        #[derive(Clone)]
        pub struct #builder_name {
            pub selects: Option<String>,
            pub is_distinct: bool,
            pub limit: Option<usize>,
            pub offset: Option<usize>,
            pub order_by: Option<String>,
            pub group_by: Option<String>,
            pub joins: Vec<String>,
            pub wheres: Vec<(String, String)>,
            pub havings: Vec<(String, String)>,
            pub bindings: Vec<rust_eloquent::EloquentValue>,
            pub with_trashed: bool,
            pub only_trashed: bool,
            #[cfg(feature = "redis")]
            pub remember_ttl: Option<usize>,
            #(#relation_flags)*
        }

        impl rust_eloquent::schema::SubqueryBuilder for #builder_name {
            fn to_sql(&self) -> String {
                self.to_sql()
            }
            fn bindings(&self) -> &Vec<rust_eloquent::EloquentValue> {
                &self.bindings
            }
        }

        impl #builder_name {
            pub fn new() -> Self {
                Self {
                    selects: None,
                    is_distinct: false,
                    limit: None,
                    offset: None,
                    order_by: None,
                    group_by: None,
                    joins: vec![],
                    wheres: vec![],
                    havings: vec![],
                    bindings: vec![],
                    with_trashed: false,
                    only_trashed: false,
                    #[cfg(feature = "redis")]
                    remember_ttl: None,
                    #(#relation_inits)*
                }
            }

            #(#relation_methods)*

            #[cfg(feature = "redis")]
            pub fn remember(mut self, seconds: usize) -> Self {
                self.remember_ttl = Some(seconds);
                self
            }

            /// Executes a raw WHERE clause. 
            /// WARNING: Do not pass user input directly into `query` as it can cause SQL Injection.
            /// Always use parameterized bindings when dealing with user data.
            pub fn where_raw(mut self, query: &str) -> Self {
                self.wheres.push(("AND".to_string(), query.to_string()));
                self
            }

            pub fn or_where_raw(mut self, query: &str) -> Self {
                self.wheres.push(("OR".to_string(), query.to_string()));
                self
            }

            pub fn where_exists<B: rust_eloquent::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                let sql = subquery.to_sql();
                self.wheres.push(("AND".to_string(), format!("EXISTS ({})", sql)));
                for binding in subquery.bindings() {
                    self.bindings.push(binding.clone());
                }
                self
            }

            pub fn or_where_exists<B: rust_eloquent::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                let sql = subquery.to_sql();
                self.wheres.push(("OR".to_string(), format!("EXISTS ({})", sql)));
                for binding in subquery.bindings() {
                    self.bindings.push(binding.clone());
                }
                self
            }

            /// Executes a raw SELECT clause. 
            /// WARNING: Make sure to avoid user input concatenation in the select string.
            pub fn select_raw(mut self, query: &str) -> Self {
                self.selects = Some(query.to_string());
                self
            }

            pub fn distinct(mut self) -> Self {
                self.is_distinct = true;
                self
            }

            pub fn with_trashed(mut self) -> Self {
                self.with_trashed = true;
                self
            }

            pub fn only_trashed(mut self) -> Self {
                self.only_trashed = true;
                self
            }

            pub fn join_constrained<F>(mut self, table: &str, modifier: F) -> Self
            where F: FnOnce(&mut rust_eloquent::JoinClause) -> &mut rust_eloquent::JoinClause
            {
                let mut clause = rust_eloquent::JoinClause::new("INNER");
                modifier(&mut clause);
                self.joins.push(format!("INNER JOIN {} ON {}", table, clause.to_sql()));
                for binding in clause.bindings {
                    self.bindings.push(binding);
                }
                self
            }

            pub fn join(mut self, table: &str, first: &str, operator: &str, second: &str) -> Self {
                self.joins.push(format!("INNER JOIN {} ON {} {} {}", table, first, operator, second));
                self
            }

            pub fn left_join(mut self, table: &str, first: &str, operator: &str, second: &str) -> Self {
                self.joins.push(format!("LEFT JOIN {} ON {} {} {}", table, first, operator, second));
                self
            }

            pub fn right_join(mut self, table: &str, first: &str, operator: &str, second: &str) -> Self {
                self.joins.push(format!("RIGHT JOIN {} ON {} {} {}", table, first, operator, second));
                self
            }

            pub fn where_eq<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} = ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_not_eq<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} != ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_gt<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} > ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_lt<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} < ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_like<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} LIKE ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_not_like<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} NOT LIKE ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_null(mut self, column: &str) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} IS NULL", column)));
                self
            }

            pub fn select(mut self, columns: &[&str]) -> Self {
                self.selects = Some(columns.join(", "));
                self
            }

            pub fn select_cols(mut self, cols: &[#column_enum_name]) -> Self {
                let s = cols.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", ");
                self.selects = Some(s);
                self
            }

            pub fn where_col<T: Into<rust_eloquent::EloquentValue>>(mut self, col: #column_enum_name, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} = ?", col.as_str())));
                self.bindings.push(value.into());
                self
            }

            pub fn order_by_col(mut self, col: #column_enum_name) -> Self {
                self.order_by = Some(col.as_str().to_string());
                self
            }

            pub fn order_by_desc_col(mut self, col: #column_enum_name) -> Self {
                self.order_by = Some(format!("{} DESC", col.as_str()));
                self
            }

            pub fn where_not_null(mut self, column: &str) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} IS NOT NULL", column)));
                self
            }

            /// WARNING: Ensure `column` does not contain user input to prevent SQL Injection.
            pub fn where_in<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, values: Vec<T>) -> Self {
                if values.is_empty() { return self; }
                let placeholders = vec!["?"; values.len()].join(", ");
                self.wheres.push(("AND".to_string(), format!("{} IN ({})", column, placeholders)));
                for v in values { self.bindings.push(v.into()); }
                self
            }

            pub fn where_not_in<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, values: Vec<T>) -> Self {
                if values.is_empty() { return self; }
                let placeholders = vec!["?"; values.len()].join(", ");
                self.wheres.push(("AND".to_string(), format!("{} NOT IN ({})", column, placeholders)));
                for v in values { self.bindings.push(v.into()); }
                self
            }

            pub fn where_between<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, min: T, max: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} BETWEEN ? AND ?", column)));
                self.bindings.push(min.into());
                self.bindings.push(max.into());
                self
            }

            pub fn where_not_between<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, min: T, max: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} NOT BETWEEN ? AND ?", column)));
                self.bindings.push(min.into());
                self.bindings.push(max.into());
                self
            }

            pub fn where_column(mut self, first: &str, second: &str) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} = {}", first, second)));
                self
            }

            pub fn or_where<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} = ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_not_eq<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} != ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_gt<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} > ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_lt<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} < ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_like<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} LIKE ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_null(mut self, column: &str) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} IS NULL", column)));
                self
            }

            pub fn or_where_not_null(mut self, column: &str) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} IS NOT NULL", column)));
                self
            }

            /// WARNING: Ensure `column` does not contain user input to prevent SQL Injection.
            pub fn or_where_in<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, values: Vec<T>) -> Self {
                if values.is_empty() { return self; }
                let placeholders = vec!["?"; values.len()].join(", ");
                self.wheres.push(("OR".to_string(), format!("{} IN ({})", column, placeholders)));
                for v in values { self.bindings.push(v.into()); }
                self
            }

            pub fn or_where_between<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, min: T, max: T) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} BETWEEN ? AND ?", column)));
                self.bindings.push(min.into());
                self.bindings.push(max.into());
                self
            }

            pub fn group_by(mut self, column: &str) -> Self {
                self.group_by = Some(column.to_string());
                self
            }

            pub fn order_by(mut self, column: &str) -> Self {
                self.order_by = Some(format!("{} ASC", column));
                self
            }

            pub fn order_by_desc(mut self, column: &str) -> Self {
                self.order_by = Some(format!("{} DESC", column));
                self
            }

            pub fn limit(mut self, value: usize) -> Self {
                self.limit = Some(value);
                self
            }

            pub fn offset(mut self, value: usize) -> Self {
                self.offset = Some(value);
                self
            }

            /// WARNING: This generates the raw SQL query. Ensure all dynamic table names and column names are validated.
            pub fn to_sql(&self) -> String {
                let select_clause = match &self.selects {
                    Some(s) => s.as_str(),
                    None => "*",
                };
                let distinct = if self.is_distinct { "DISTINCT " } else { "" };
                
                // Estimate capacity: SELECT + FROM + table + joins + wheres
                let estimated_capacity = 50 + #table_name.len() + self.joins.iter().map(|j| j.len() + 1).sum::<usize>() 
                    + self.wheres.iter().map(|(o, c)| o.len() + c.len() + 4).sum::<usize>();
                let mut sql = String::with_capacity(estimated_capacity);
                
                sql.push_str("SELECT ");
                if self.is_distinct {
                    sql.push_str("DISTINCT ");
                }
                sql.push_str(select_clause);
                sql.push_str(" FROM ");
                sql.push_str(#table_name);

                for join in &self.joins {
                    sql.push(' ');
                    sql.push_str(join);
                }

                let mut first_where = true;
                if !self.wheres.is_empty() {
                    sql.push_str(" WHERE ");
                    for (op, cond) in &self.wheres {
                        if first_where {
                            sql.push('(');
                            sql.push_str(cond);
                            sql.push(')');
                            first_where = false;
                        } else {
                            sql.push(' ');
                            sql.push_str(op);
                            sql.push_str(" (");
                            sql.push_str(cond);
                            sql.push(')');
                        }
                    }
                }

                if #has_soft_deletes && !self.with_trashed {
                    if first_where {
                        sql.push_str(" WHERE ");
                    } else {
                        sql.push_str(" AND ");
                    }
                    if self.only_trashed {
                        sql.push_str("deleted_at IS NOT NULL");
                    } else {
                        sql.push_str("deleted_at IS NULL");
                    }
                }

                if let Some(group) = &self.group_by {
                    sql.push_str(" GROUP BY ");
                    sql.push_str(group);
                }

                let mut first_having = true;
                if !self.havings.is_empty() {
                    sql.push_str(" HAVING ");
                    for (op, cond) in &self.havings {
                        if first_having {
                            sql.push('(');
                            sql.push_str(cond);
                            sql.push(')');
                            first_having = false;
                        } else {
                            sql.push(' ');
                            sql.push_str(op);
                            sql.push_str(" (");
                            sql.push_str(cond);
                            sql.push(')');
                        }
                    }
                }

                if let Some(order) = &self.order_by {
                    sql.push_str(" ORDER BY ");
                    sql.push_str(order);
                }

                if let Some(limit) = self.limit {
                    sql.push_str(" LIMIT ");
                    sql.push_str(&limit.to_string());
                }
                if let Some(offset) = self.offset {
                    sql.push_str(" OFFSET ");
                    sql.push_str(&offset.to_string());
                }

                sql
            }

            pub async fn get(&self) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::read_pool();
                self.get_with_tx_internal(pool).await
            }

            pub async fn get_with_tx(&self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::EloquentDatabase>) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> {
                self.get_with_tx_internal(&mut **tx).await
            }

            async fn get_with_tx_internal<'e, E>(&self, executor: E) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> 
            where E: rust_eloquent::sqlx::Executor<'e, Database = rust_eloquent::EloquentDatabase>
            {
                let query_str = self.to_sql();

                #[cfg(feature = "redis")]
                {
                    if let Some(ttl) = self.remember_ttl {
                        use rust_eloquent::redis::AsyncCommands;
                        let cache_key = format!("eloquent:cache:{}:{:?}", #table_name, (&query_str, &self.bindings));
                        let mut conn = rust_eloquent::Eloquent::redis_manager();
                        if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                            if !cached_data.is_empty() {
                                if let Ok(mut results) = #name::from_cache_json_array(&cached_data) {
                                    #hook_after_fetch
                                    #eager_loads
                                    return Ok(results);
                                }
                            }
                        }
                    }
                }

                if rust_eloquent::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | Bindings: {:?}", query_str, self.bindings);
                }
                let mut results: Vec<#name> = {
                    use rust_eloquent::sqlx::query_builder::QueryBuilder;
                    let mut query_builder = QueryBuilder::new(&query_str);
                    for binding in &self.bindings {
                        match binding {
                            rust_eloquent::EloquentValue::String(s) => { query_builder.push_bind(s.clone()); }
                            rust_eloquent::EloquentValue::Int(i) => { query_builder.push_bind(*i); }
                            rust_eloquent::EloquentValue::Float(f) => { query_builder.push_bind(*f); }
                            rust_eloquent::EloquentValue::Bool(b) => { query_builder.push_bind(*b); }
                        }
                    }
                    query_builder.build_query_as::<#name>().fetch_all(executor).await?
                };
                
                #[cfg(feature = "redis")]
                {
                    if let Some(ttl) = self.remember_ttl {
                        use rust_eloquent::redis::AsyncCommands;
                        let cache_key = format!("eloquent:cache:{}:{:?}", #table_name, (&query_str, &self.bindings));
                        let serialized = #name::to_cache_json_array(&results);
                        let mut conn = rust_eloquent::Eloquent::redis_manager();
                        let _: Result<(), rust_eloquent::redis::RedisError> = conn.set_ex(&cache_key, serialized, ttl as u64).await;
                    }
                }

                #hook_after_fetch
                #eager_loads
                Ok(results)
            }

            pub async fn first(&self) -> Result<Option<#name>, rust_eloquent::sqlx::Error> {
                let mut builder = self.clone();
                builder.limit = Some(1);
                let results = builder.get().await?;
                Ok(results.into_iter().next())
            }

            pub async fn first_with_tx(&self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::EloquentDatabase>) -> Result<Option<#name>, rust_eloquent::sqlx::Error> {
                let mut builder = self.clone();
                builder.limit = Some(1);
                let results = builder.get_with_tx(tx).await?;
                Ok(results.into_iter().next())
            }

            pub async fn paginate(&self, page: usize, per_page: usize) -> Result<rust_eloquent::PaginationResult<#name>, rust_eloquent::sqlx::Error> {
                let total_builder = Self {
                    selects: Some("COUNT(*)".to_string()),
                    limit: None,
                    offset: None,
                    order_by: None,
                    is_distinct: self.is_distinct.clone(),
                    joins: self.joins.clone(),
                    wheres: self.wheres.clone(),
                    havings: self.havings.clone(),
                    bindings: self.bindings.clone(),
                    group_by: self.group_by.clone(),
                    with_trashed: self.with_trashed,
                    only_trashed: self.only_trashed,
                    ..self.clone()
                };
                
                let query_str = total_builder.to_sql();
                if rust_eloquent::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | Bindings: {:?}", query_str, total_builder.bindings);
                }
                let pool = rust_eloquent::Eloquent::read_pool();
                let total_row: (i64,) = {
                    use rust_eloquent::sqlx::query_builder::QueryBuilder;
                    let mut query_builder = QueryBuilder::new(&query_str);
                    for binding in &total_builder.bindings {
                        match binding {
                            rust_eloquent::EloquentValue::String(s) => { query_builder.push_bind(s.clone()); }
                            rust_eloquent::EloquentValue::Int(i) => { query_builder.push_bind(*i); }
                            rust_eloquent::EloquentValue::Float(f) => { query_builder.push_bind(*f); }
                            rust_eloquent::EloquentValue::Bool(b) => { query_builder.push_bind(*b); }
                        }
                    }
                    query_builder.build_query_as::<(i64,)>().fetch_one(pool).await?
                };
                let total = total_row.0;
                let last_page = (total as f64 / per_page as f64).ceil() as usize;
                
                let mut data_builder = self.clone();
                data_builder.limit = Some(per_page);
                if page > 1 {
                    data_builder.offset = Some((page - 1) * per_page);
                }
                let data = data_builder.get().await?;
                
                Ok(rust_eloquent::PaginationResult {
                    data,
                    total,
                    per_page,
                    current_page: if page == 0 { 1 } else { page },
                    last_page,
                })
            }

            pub async fn count(&self) -> Result<i64, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::read_pool();
                let mut builder = self.clone();
                builder.selects = Some("COUNT(*)".to_string());
                builder.limit = None;
                builder.offset = None;
                builder.order_by = None;
                let query_str = builder.to_sql();
                if rust_eloquent::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | Bindings: {:?}", query_str, builder.bindings);
                }
                
                let row: (i64,) = {
                    use rust_eloquent::sqlx::query_builder::QueryBuilder;
                    let mut query_builder = QueryBuilder::new(&query_str);
                    for binding in &builder.bindings {
                        match binding {
                            rust_eloquent::EloquentValue::String(s) => { query_builder.push_bind(s.clone()); }
                            rust_eloquent::EloquentValue::Int(i) => { query_builder.push_bind(*i); }
                            rust_eloquent::EloquentValue::Float(f) => { query_builder.push_bind(*f); }
                            rust_eloquent::EloquentValue::Bool(b) => { query_builder.push_bind(*b); }
                        }
                    }
                    query_builder.build_query_as::<(i64,)>().fetch_one(pool).await?
                };
                Ok(row.0)
            }

            pub async fn chunk<F, Fut>(&self, size: usize, mut handler: F) -> Result<(), rust_eloquent::sqlx::Error>
            where
                F: FnMut(Vec<#name>) -> Fut + Send,
                Fut: std::future::Future<Output = ()> + Send,
            {
                let mut page = 1;
                loop {
                    let mut builder = self.clone();
                    builder.limit = Some(size);
                    builder.offset = Some((page - 1) * size);
                    let results = builder.get().await?;
                    let count = results.len();
                    if count == 0 { break; }
                    handler(results).await;
                    if count < size { break; }
                    page += 1;
                }
                Ok(())
            }

            pub async fn chunk_with_tx<F, Fut>(&self, size: usize, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::EloquentDatabase>, mut handler: F) -> Result<(), rust_eloquent::sqlx::Error>
            where
                F: FnMut(Vec<#name>) -> Fut + Send,
                Fut: std::future::Future<Output = ()> + Send,
            {
                let mut page = 1;
                loop {
                    let mut builder = self.clone();
                    builder.limit = Some(size);
                    builder.offset = Some((page - 1) * size);
                    let results = builder.get_with_tx(tx).await?;
                    let count = results.len();
                    if count == 0 { break; }
                    handler(results).await;
                    if count < size { break; }
                    page += 1;
                }
                Ok(())
            }

            pub async fn delete_all(&self) -> Result<u64, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                self.delete_all_with_tx_internal(pool).await
            }

            pub async fn delete_all_with_tx(&self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::EloquentDatabase>) -> Result<u64, rust_eloquent::sqlx::Error> {
                self.delete_all_with_tx_internal(&mut **tx).await
            }

            async fn delete_all_with_tx_internal<'e, E>(&self, executor: E) -> Result<u64, rust_eloquent::sqlx::Error> 
            where E: rust_eloquent::sqlx::Executor<'e, Database = rust_eloquent::EloquentDatabase>
            {
                #delete_all_logic
                
                if !self.wheres.is_empty() {
                    query_str.push_str(" WHERE ");
                    let mut first = true;
                    for (operator, condition) in &self.wheres {
                        if first {
                            query_str.push_str(&format!("({})", condition));
                            first = false;
                        } else {
                            query_str.push_str(&format!(" {} ({})", operator, condition));
                        }
                    }
                }

                let result = {
                    use rust_eloquent::sqlx::query_builder::QueryBuilder;
                    let mut query_builder = QueryBuilder::new(&query_str);
                    for binding in &self.bindings {
                        match binding {
                            rust_eloquent::EloquentValue::String(s) => { query_builder.push_bind(s.clone()); }
                            rust_eloquent::EloquentValue::Int(i) => { query_builder.push_bind(*i); }
                            rust_eloquent::EloquentValue::Float(f) => { query_builder.push_bind(*f); }
                            rust_eloquent::EloquentValue::Bool(b) => { query_builder.push_bind(*b); }
                        }
                    }
                    let query = query_builder.build();
                    query.execute(executor).await?
                };
                Ok(result.rows_affected())
            }

            pub async fn pluck_string(&self, column: &str) -> Result<Vec<String>, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::read_pool();
                let mut builder = self.clone();
                builder.selects = Some(column.to_string());
                let query_str = builder.to_sql();
                let rows: Vec<(String,)> = {
                    use rust_eloquent::sqlx::query_builder::QueryBuilder;
                    let mut query_builder = QueryBuilder::new(&query_str);
                    for binding in &builder.bindings {
                        match binding {
                            rust_eloquent::EloquentValue::String(s) => { query_builder.push_bind(s.clone()); }
                            rust_eloquent::EloquentValue::Int(i) => { query_builder.push_bind(*i); }
                            rust_eloquent::EloquentValue::Float(f) => { query_builder.push_bind(*f); }
                            rust_eloquent::EloquentValue::Bool(b) => { query_builder.push_bind(*b); }
                        }
                    }
                    query_builder.build_query_as::<(String,)>().fetch_all(pool).await?
                };
                Ok(rows.into_iter().map(|(s,)| s).collect())
            }

            pub async fn pluck_i32(&self, column: &str) -> Result<Vec<i32>, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::read_pool();
                let mut builder = self.clone();
                builder.selects = Some(column.to_string());
                let query_str = builder.to_sql();
                let rows: Vec<(i32,)> = {
                    use rust_eloquent::sqlx::query_builder::QueryBuilder;
                    let mut query_builder = QueryBuilder::new(&query_str);
                    for binding in &builder.bindings {
                        match binding {
                            rust_eloquent::EloquentValue::String(s) => { query_builder.push_bind(s.clone()); }
                            rust_eloquent::EloquentValue::Int(i) => { query_builder.push_bind(*i); }
                            rust_eloquent::EloquentValue::Float(f) => { query_builder.push_bind(*f); }
                            rust_eloquent::EloquentValue::Bool(b) => { query_builder.push_bind(*b); }
                        }
                    }
                    query_builder.build_query_as::<(i32,)>().fetch_all(pool).await?
                };
                Ok(rows.into_iter().map(|(s,)| s).collect())
            }

            #(#magic_methods)*
        }
    }
}
