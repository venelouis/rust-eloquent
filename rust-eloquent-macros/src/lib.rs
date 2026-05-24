extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(Eloquent)]
pub fn eloquent_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let table_name = format!("{}s", name.to_string().to_lowercase());
    
    let builder_name_str = format!("{}QueryBuilder", name);
    let builder_name = syn::Ident::new(&builder_name_str, name.span());

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("Eloquent macro only supports structs with named fields"),
        },
        _ => panic!("Eloquent macro can only be used on structs"),
    };

    let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();

    let insert_columns = field_names.iter().filter(|&&ident| ident != "id").map(|ident| ident.to_string()).collect::<Vec<_>>().join(", ");
    let insert_placeholders = field_names.iter().filter(|&&ident| ident != "id").map(|_| "?").collect::<Vec<_>>().join(", ");
    let update_sets = field_names.iter().filter(|&&ident| ident != "id").map(|ident| format!("{} = ?", ident)).collect::<Vec<_>>().join(", ");

    let bind_inserts: Vec<_> = field_names.iter().filter(|&&ident| ident != "id").map(|ident| quote! { .bind(self.#ident.clone()) }).collect();
    let bind_updates: Vec<_> = field_names.iter().filter(|&&ident| ident != "id").map(|ident| quote! { .bind(self.#ident.clone()) }).collect();

    // ==========================================
    // MAGIC METHODS GENERATION
    // ==========================================
    let magic_methods: Vec<_> = field_names.iter().map(|ident| {
        let field_name_str = ident.to_string();
        let where_method = quote::format_ident!("where_{}", ident);
        let where_not_method = quote::format_ident!("where_not_{}", ident);
        let or_where_method = quote::format_ident!("or_where_{}", ident);
        let order_by_method = quote::format_ident!("order_by_{}", ident);
        let order_by_desc_method = quote::format_ident!("order_by_{}_desc", ident);

        quote! {
            pub fn #where_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.where_eq(#field_name_str, value)
            }
            pub fn #where_not_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.where_not_eq(#field_name_str, value)
            }
            pub fn #or_where_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.or_where(#field_name_str, value)
            }
            pub fn #order_by_method(self) -> Self {
                self.order_by(#field_name_str)
            }
            pub fn #order_by_desc_method(self) -> Self {
                self.order_by_desc(#field_name_str)
            }
        }
    }).collect();

    let expanded = quote! {
        #[rust_eloquent::async_trait]
        impl rust_eloquent::EloquentModel for #name {
            fn table_name() -> &'static str {
                #table_name
            }
        }

        // ==========================================
        // QUERY BUILDER
        // ==========================================
        pub struct #builder_name {
            pub selects: Option<String>,
            pub is_distinct: bool,
            pub joins: Vec<String>,
            pub wheres: Vec<(String, String)>,
            pub havings: Vec<(String, String)>,
            pub bindings: Vec<rust_eloquent::EloquentValue>,
            pub group_by: Option<String>,
            pub order_by: Option<String>,
            pub limit: Option<usize>,
            pub offset: Option<usize>,
        }

        impl #builder_name {
            pub fn new() -> Self {
                Self {
                    selects: None,
                    is_distinct: false,
                    joins: vec![],
                    wheres: vec![],
                    havings: vec![],
                    bindings: vec![],
                    group_by: None,
                    order_by: None,
                    limit: None,
                    offset: None,
                }
            }

            // --- Selects & Distinct ---
            pub fn select(mut self, columns: Vec<&str>) -> Self {
                self.selects = Some(columns.join(", "));
                self
            }

            pub fn distinct(mut self) -> Self {
                self.is_distinct = true;
                self
            }

            // --- Aliases ---
            pub fn take(self, value: usize) -> Self { self.limit(value) }
            pub fn skip(self, value: usize) -> Self { self.offset(value) }
            pub fn latest(self, column: &str) -> Self { self.order_by_desc(column) }
            pub fn oldest(self, column: &str) -> Self { self.order_by(column) }

            // --- Joins ---
            pub fn join(mut self, table: &str, first: &str, operator: &str, second: &str) -> Self {
                self.joins.push(format!("JOIN {} ON {} {} {}", table, first, operator, second));
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
            pub fn cross_join(mut self, table: &str) -> Self {
                self.joins.push(format!("CROSS JOIN {}", table));
                self
            }

            // --- Column Comparisons ---
            pub fn where_column(mut self, first: &str, second: &str) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} = {}", first, second)));
                self
            }
            pub fn where_column_op(mut self, first: &str, operator: &str, second: &str) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} {} {}", first, operator, second)));
                self
            }
            pub fn or_where_column(mut self, first: &str, second: &str) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} = {}", first, second)));
                self
            }
            pub fn or_where_column_op(mut self, first: &str, operator: &str, second: &str) -> Self {
                self.wheres.push(("OR".to_string(), format!("{} {} {}", first, operator, second)));
                self
            }

            // --- Raw Queries ---
            pub fn where_raw<T: Into<rust_eloquent::EloquentValue>>(mut self, sql: &str, bindings: Vec<T>) -> Self {
                self.wheres.push(("AND".to_string(), sql.to_string()));
                for v in bindings { self.bindings.push(v.into()); }
                self
            }
            pub fn or_where_raw<T: Into<rust_eloquent::EloquentValue>>(mut self, sql: &str, bindings: Vec<T>) -> Self {
                self.wheres.push(("OR".to_string(), sql.to_string()));
                for v in bindings { self.bindings.push(v.into()); }
                self
            }
            pub fn select_raw(mut self, sql: &str) -> Self {
                self.selects = Some(sql.to_string());
                self
            }
            pub fn order_by_raw(mut self, sql: &str) -> Self {
                self.order_by = Some(sql.to_string());
                self
            }
            pub fn group_by_raw(mut self, sql: &str) -> Self {
                self.group_by = Some(sql.to_string());
                self
            }

            // --- Havings ---
            pub fn having<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, operator: &str, value: T) -> Self {
                self.havings.push(("AND".to_string(), format!("{} {} ?", column, operator)));
                self.bindings.push(value.into());
                self
            }
            pub fn or_having<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, operator: &str, value: T) -> Self {
                self.havings.push(("OR".to_string(), format!("{} {} ?", column, operator)));
                self.bindings.push(value.into());
                self
            }
            pub fn having_raw<T: Into<rust_eloquent::EloquentValue>>(mut self, sql: &str, bindings: Vec<T>) -> Self {
                self.havings.push(("AND".to_string(), sql.to_string()));
                for v in bindings { self.bindings.push(v.into()); }
                self
            }

            // --- Wheres (AND) ---
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
            pub fn where_gte<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} >= ?", column)));
                self.bindings.push(value.into());
                self
            }
            pub fn where_lte<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} <= ?", column)));
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
            pub fn where_not_null(mut self, column: &str) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} IS NOT NULL", column)));
                self
            }
            pub fn where_in<T: Into<rust_eloquent::EloquentValue>>(mut self, column: &str, values: Vec<T>) -> Self {
                if values.is_empty() {
                    self.wheres.push(("AND".to_string(), "1 = 0".to_string()));
                    return self;
                }
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

            // --- Wheres (OR) ---
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

            // --- Grouping, Order and Limits ---
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

            // --- Utilities ---
            pub fn to_sql(&self) -> String {
                let mut query_str = String::new();
                query_str.push_str("SELECT ");
                
                if self.is_distinct {
                    query_str.push_str("DISTINCT ");
                }
                
                if let Some(ref selects) = self.selects {
                    query_str.push_str(selects);
                } else {
                    query_str.push_str("*");
                }
                
                query_str.push_str(&format!(" FROM {}", #table_name));

                if !self.joins.is_empty() {
                    for join in &self.joins {
                        query_str.push_str(" ");
                        query_str.push_str(join);
                    }
                }

                if !self.wheres.is_empty() {
                    query_str.push_str(" WHERE ");
                    for (i, (operator, condition)) in self.wheres.iter().enumerate() {
                        if i > 0 {
                            query_str.push_str(&format!(" {} ", operator));
                        }
                        query_str.push_str(condition);
                    }
                }

                if let Some(ref group) = self.group_by {
                    query_str.push_str(" GROUP BY ");
                    query_str.push_str(group);
                }

                if !self.havings.is_empty() {
                    query_str.push_str(" HAVING ");
                    for (i, (operator, condition)) in self.havings.iter().enumerate() {
                        if i > 0 {
                            query_str.push_str(&format!(" {} ", operator));
                        }
                        query_str.push_str(condition);
                    }
                }

                if let Some(ref order) = self.order_by {
                    query_str.push_str(" ORDER BY ");
                    query_str.push_str(order);
                }

                if let Some(limit) = self.limit {
                    query_str.push_str(&format!(" LIMIT {}", limit));
                }

                if let Some(offset) = self.offset {
                    query_str.push_str(&format!(" OFFSET {}", offset));
                }
                
                query_str
            }

            // --- Executors ---
            pub async fn get(&self) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let query_str = self.to_sql();

                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &self.bindings {
                    match binding {
                        rust_eloquent::EloquentValue::String(s) => rust_eloquent::sqlx::Arguments::add(&mut args, s.clone()).unwrap(),
                        rust_eloquent::EloquentValue::Int(i) => rust_eloquent::sqlx::Arguments::add(&mut args, *i).unwrap(),
                        rust_eloquent::EloquentValue::Float(f) => rust_eloquent::sqlx::Arguments::add(&mut args, *f).unwrap(),
                        rust_eloquent::EloquentValue::Bool(b) => rust_eloquent::sqlx::Arguments::add(&mut args, *b).unwrap(),
                    }
                }

                rust_eloquent::sqlx::query_as_with::<_, #name, _>(&query_str, args)
                    .fetch_all(pool)
                    .await
            }

            pub async fn first(&self) -> Result<#name, rust_eloquent::sqlx::Error> {
                let mut builder = Self {
                    selects: self.selects.clone(),
                    is_distinct: self.is_distinct.clone(),
                    joins: self.joins.clone(),
                    wheres: self.wheres.clone(),
                    havings: self.havings.clone(),
                    bindings: self.bindings.clone(),
                    group_by: self.group_by.clone(),
                    order_by: self.order_by.clone(),
                    limit: Some(1),
                    offset: self.offset.clone(),
                };
                
                let result = builder.get().await?;
                if result.is_empty() {
                    Err(rust_eloquent::sqlx::Error::RowNotFound)
                } else {
                    Ok(result.into_iter().next().unwrap())
                }
            }

            pub async fn count(&self) -> Result<i64, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let mut query_str = format!("SELECT COUNT(*) FROM {}", #table_name);
                
                if !self.joins.is_empty() {
                    for join in &self.joins {
                        query_str.push_str(" ");
                        query_str.push_str(join);
                    }
                }
                
                if !self.wheres.is_empty() {
                    query_str.push_str(" WHERE ");
                    for (i, (operator, condition)) in self.wheres.iter().enumerate() {
                        if i > 0 {
                            query_str.push_str(&format!(" {} ", operator));
                        }
                        query_str.push_str(condition);
                    }
                }

                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &self.bindings {
                    match binding {
                        rust_eloquent::EloquentValue::String(s) => rust_eloquent::sqlx::Arguments::add(&mut args, s.clone()).unwrap(),
                        rust_eloquent::EloquentValue::Int(i) => rust_eloquent::sqlx::Arguments::add(&mut args, *i).unwrap(),
                        rust_eloquent::EloquentValue::Float(f) => rust_eloquent::sqlx::Arguments::add(&mut args, *f).unwrap(),
                        rust_eloquent::EloquentValue::Bool(b) => rust_eloquent::sqlx::Arguments::add(&mut args, *b).unwrap(),
                    }
                }

                let row: (i64,) = rust_eloquent::sqlx::query_as_with(&query_str, args).fetch_one(pool).await?;
                Ok(row.0)
            }

            pub async fn delete_all(&self) -> Result<u64, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let mut query_str = format!("DELETE FROM {}", #table_name);
                
                if !self.wheres.is_empty() {
                    query_str.push_str(" WHERE ");
                    for (i, (operator, condition)) in self.wheres.iter().enumerate() {
                        if i > 0 {
                            query_str.push_str(&format!(" {} ", operator));
                        }
                        query_str.push_str(condition);
                    }
                }

                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &self.bindings {
                    match binding {
                        rust_eloquent::EloquentValue::String(s) => rust_eloquent::sqlx::Arguments::add(&mut args, s.clone()).unwrap(),
                        rust_eloquent::EloquentValue::Int(i) => rust_eloquent::sqlx::Arguments::add(&mut args, *i).unwrap(),
                        rust_eloquent::EloquentValue::Float(f) => rust_eloquent::sqlx::Arguments::add(&mut args, *f).unwrap(),
                        rust_eloquent::EloquentValue::Bool(b) => rust_eloquent::sqlx::Arguments::add(&mut args, *b).unwrap(),
                    }
                }

                let result = rust_eloquent::sqlx::query_with(&query_str, args).execute(pool).await?;
                Ok(result.rows_affected())
            }

            // --- Magic Dynamic Methods ---
            #(#magic_methods)*
        }

        // ==========================================
        // ACTIVE RECORD METHODS
        // ==========================================
        impl #name {
            /// Initialize a new Query Builder for this model
            pub fn query() -> #builder_name {
                #builder_name::new()
            }

            /// Find a record by its primary key (ID)
            pub async fn find(id: i32) -> Result<Self, rust_eloquent::sqlx::Error> {
                Self::query().where_eq("id", id).first().await
            }

            /// Retrieve all records from the table
            pub async fn all() -> Result<Vec<Self>, rust_eloquent::sqlx::Error> {
                Self::query().get().await
            }

            /// Insert a new record into the database
            pub async fn insert(&mut self) -> Result<(), rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let driver = rust_eloquent::Eloquent::driver();

                if driver == "postgres" {
                    let query = format!("INSERT INTO {} ({}) VALUES ({}) RETURNING id", #table_name, #insert_columns, #insert_placeholders);
                    let row = rust_eloquent::sqlx::query(&query)
                        #(#bind_inserts)*
                        .fetch_one(pool)
                        .await?;
                    self.id = rust_eloquent::sqlx::Row::try_get(&row, "id")?;
                } else {
                    let query = format!("INSERT INTO {} ({}) VALUES ({})", #table_name, #insert_columns, #insert_placeholders);
                    let result = rust_eloquent::sqlx::query(&query)
                        #(#bind_inserts)*
                        .execute(pool)
                        .await?;
                    
                    self.id = result.last_insert_id().unwrap_or(0) as i32;
                }
                
                Ok(())
            }

            /// Update an existing record in the database
            pub async fn update(&self) -> Result<(), rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let query = format!("UPDATE {} SET {} WHERE id = ?", #table_name, #update_sets);
                
                rust_eloquent::sqlx::query(&query)
                    #(#bind_updates)*
                    .bind(self.id)
                    .execute(pool)
                    .await?;
                    
                Ok(())
            }

            /// Save the model to the database
            pub async fn save(&mut self) -> Result<(), rust_eloquent::sqlx::Error> {
                if self.id == 0 {
                    self.insert().await
                } else {
                    self.update().await
                }
            }

            /// Delete the record from the database
            pub async fn delete(&self) -> Result<(), rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let query = format!("DELETE FROM {} WHERE id = ?", #table_name);
                
                rust_eloquent::sqlx::query(&query)
                    .bind(self.id)
                    .execute(pool)
                    .await?;
                    
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}
