use quote::quote;
use proc_macro2::TokenStream;
use crate::parser::ParsedModel;

pub fn generate(
    parsed: &ParsedModel,
    relationship_methods: &[TokenStream],
) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let builder_name = quote::format_ident!("{}QueryBuilder", name);
    let observer_trait_name = quote::format_ident!("{}Observer", name);
    
    let normal_fields = &parsed.normal_fields;
    let hidden_fields = &parsed.hidden_fields;
    let has_soft_deletes = parsed.has_soft_deletes;

    let mut relation_field_idents = vec![];
    for rel in &parsed.relations {
        relation_field_idents.push(rel.field_name.clone());
    }

    let hook_before_save = if !parsed.before_save.is_empty() { let method = syn::Ident::new(&parsed.before_save, name.span()); quote! { self.#method().await?; } } else { quote! {} };
    let hook_after_save = if !parsed.after_save.is_empty() { let method = syn::Ident::new(&parsed.after_save, name.span()); quote! { self.#method().await?; } } else { quote! {} };
    let hook_before_delete = if !parsed.before_delete.is_empty() { let method = syn::Ident::new(&parsed.before_delete, name.span()); quote! { self.#method().await?; } } else { quote! {} };
    let hook_after_delete = if !parsed.after_delete.is_empty() { let method = syn::Ident::new(&parsed.after_delete, name.span()); quote! { self.#method().await?; } } else { quote! {} };

    let global_scope_logic = if !parsed.global_scope.is_empty() {
        let method = syn::Ident::new(&parsed.global_scope, name.span());
        quote! { builder = builder.#method(); }
    } else {
        quote! {}
    };

    let mut insert_columns = vec![];
    let mut insert_placeholders = vec![];
    let mut bind_inserts = vec![];
    
    let mut update_sets = vec![];
    let mut bind_updates = vec![];
    
    let mut to_json_fields = vec![];

    for field_name in normal_fields {
        let field_name_str = field_name.to_string();
        
        if field_name_str != "id" {
            insert_columns.push(field_name_str.clone());
            insert_placeholders.push("?");
            bind_inserts.push(quote! { .bind(self.#field_name.clone()) });
            
            update_sets.push(format!("{} = ?", field_name_str));
            bind_updates.push(quote! { .bind(self.#field_name.clone()) });
        }
        
        if !hidden_fields.contains(field_name) {
            to_json_fields.push(quote! {
                map.insert(#field_name_str.to_string(), rust_eloquent::serde_json::json!(self.#field_name));
            });
        }
    }



    let insert_columns_str = insert_columns.join(", ");
    let insert_placeholders_str = insert_placeholders.join(", ");
    let update_sets_str = update_sets.join(", ");

    let delete_logic = if has_soft_deletes {
        quote! {
            use rust_eloquent::sqlx::query_builder::QueryBuilder;
            let mut query_builder = QueryBuilder::new("UPDATE ");
            query_builder.push(#table_name);
            query_builder.push(" SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?");
            let query = query_builder.build();
            query.bind(self.id).execute(pool).await?;
        }
    } else {
        quote! {
            use rust_eloquent::sqlx::query_builder::QueryBuilder;
            let mut query_builder = QueryBuilder::new("DELETE FROM ");
            query_builder.push(#table_name);
            query_builder.push(" WHERE id = ?");
            let query = query_builder.build();
            query.bind(self.id).execute(pool).await?;
        }
    };

    let column_enum_name = quote::format_ident!("{}Column", name);
    let column_variants: Vec<_> = normal_fields.iter().map(|ident| {
        let name_str = ident.to_string();
        let mut chars = name_str.chars();
        let mut camel = match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        };
        camel = camel.split('_').map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }).collect();
        quote::format_ident!("{}", camel)
    }).collect();

    let column_to_string: Vec<_> = normal_fields.iter().zip(column_variants.iter()).map(|(ident, variant)| {
        let field_name_str = ident.to_string();
        quote! { #column_enum_name::#variant => #field_name_str }
    }).collect();

    let enum_def = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #column_enum_name {
            #(#column_variants),*
        }
        impl #column_enum_name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#column_to_string),*
                }
            }
        }
    };



    quote! {
        #enum_def

        #[rust_eloquent::async_trait]
        impl rust_eloquent::EloquentModel for #name {
            fn table_name() -> &'static str {
                #table_name
            }
        }

        impl #name {
            #(#relationship_methods)*

            pub fn from_json(json_str: &str) -> Result<Self, rust_eloquent::serde_json::Error> {
                let value: rust_eloquent::serde_json::Value = rust_eloquent::serde_json::from_str(json_str)?;
                Self::from_json_value(value)
            }

            pub fn from_json_value(value: rust_eloquent::serde_json::Value) -> Result<Self, rust_eloquent::serde_json::Error> {
                Ok(Self {
                    #(
                        #normal_fields: rust_eloquent::serde_json::from_value(value[stringify!(#normal_fields)].clone())?,
                    )*
                    #(
                        #relation_field_idents: None,
                    )*
                })
            }

            pub fn from_json_array(json_str: &str) -> Result<Vec<Self>, rust_eloquent::serde_json::Error> {
                let value: rust_eloquent::serde_json::Value = rust_eloquent::serde_json::from_str(json_str)?;
                let mut results = vec![];
                if let Some(arr) = value.as_array() {
                    for item in arr {
                        results.push(Self::from_json_value(item.clone())?);
                    }
                }
                Ok(results)
            }

            pub fn to_cache_json(&self) -> String {
                let mut map = rust_eloquent::serde_json::Map::new();
                #(
                    map.insert(stringify!(#normal_fields).to_string(), rust_eloquent::serde_json::json!(self.#normal_fields));
                )*
                rust_eloquent::serde_json::Value::Object(map).to_string()
            }

            pub fn to_cache_json_array(models: &[Self]) -> String {
                let json_values: Vec<rust_eloquent::serde_json::Value> = models.iter().map(|m| {
                    let mut map = rust_eloquent::serde_json::Map::new();
                    #(
                        map.insert(stringify!(#normal_fields).to_string(), rust_eloquent::serde_json::json!(m.#normal_fields));
                    )*
                    rust_eloquent::serde_json::Value::Object(map)
                }).collect();
                rust_eloquent::serde_json::Value::Array(json_values).to_string()
            }

            pub fn from_cache_json(json_str: &str) -> Result<Self, rust_eloquent::serde_json::Error> {
                Self::from_json(json_str)
            }

            pub fn from_cache_json_array(json_str: &str) -> Result<Vec<Self>, rust_eloquent::serde_json::Error> {
                Self::from_json_array(json_str)
            }

            pub fn observe(observer: std::sync::Arc<dyn #observer_trait_name + Send + Sync>) {
                let list = Self::observers();
                let mut writer = list.write().expect("Failed to acquire write lock on observers - possible poisoning");
                writer.push(observer);
            }

            fn observers() -> &'static std::sync::RwLock<Vec<std::sync::Arc<dyn #observer_trait_name + Send + Sync>>> {
                static LIST: std::sync::OnceLock<std::sync::RwLock<Vec<std::sync::Arc<dyn #observer_trait_name + Send + Sync>>>> = std::sync::OnceLock::new();
                LIST.get_or_init(|| std::sync::RwLock::new(vec![]))
            }

            pub fn query() -> #builder_name {
                let mut builder = #builder_name::new();
                #global_scope_logic
                builder
            }

            pub async fn find(id: i32) -> Result<Option<Self>, rust_eloquent::sqlx::Error> {
                Self::query().where_eq("id", id).first().await
            }

            pub async fn find_with_tx(id: i32, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::sqlx::Any>) -> Result<Option<Self>, rust_eloquent::sqlx::Error> {
                Self::query().where_eq("id", id).first_with_tx(tx).await
            }

            pub async fn all() -> Result<Vec<Self>, rust_eloquent::sqlx::Error> {
                Self::query().get().await
            }

            pub async fn all_with_tx(tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::sqlx::Any>) -> Result<Vec<Self>, rust_eloquent::sqlx::Error> {
                Self::query().get_with_tx(tx).await
            }

            pub async fn save(&mut self) -> Result<(), rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                self.save_with_tx_internal(pool).await
            }

            pub async fn save_with_tx(&mut self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::sqlx::Any>) -> Result<(), rust_eloquent::sqlx::Error> {
                self.save_with_tx_internal(&mut **tx).await
            }

            async fn save_with_tx_internal<'e, E>(&mut self, executor: E) -> Result<(), rust_eloquent::sqlx::Error> 
            where E: rust_eloquent::sqlx::Executor<'e, Database = rust_eloquent::sqlx::Any>
            {
                let is_new = self.id == 0;
                #hook_before_save
                {
                    let observers = {
                        let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                        list.clone()
                    };
                    for obs in observers.iter() {
                        obs.saving(self).await?;
                    }
                }
                if self.id == 0 {
                    {
                        let observers = {
                            let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.creating(self).await?;
                        }
                    }
                    let driver = rust_eloquent::Eloquent::driver();
                    if driver == "postgres" {
                        use rust_eloquent::sqlx::query_builder::QueryBuilder;
                        let mut query_builder = QueryBuilder::new("INSERT INTO ");
                        query_builder.push(#table_name);
                        query_builder.push(" (");
                        query_builder.push(#insert_columns_str);
                        query_builder.push(") VALUES (");
                        query_builder.push(#insert_placeholders_str);
                        query_builder.push(") RETURNING id");
                        let query = query_builder.build();
                        if rust_eloquent::schema::is_query_log_enabled() {
                            println!("[SQL Debug] {}", query.sql());
                        }
                        let row = query
                            #(#bind_inserts)*
                            .fetch_one(executor)
                            .await?;
                        self.id = rust_eloquent::sqlx::Row::try_get(&row, "id")?;
                    } else {
                        use rust_eloquent::sqlx::query_builder::QueryBuilder;
                        let mut query_builder = QueryBuilder::new("INSERT INTO ");
                        query_builder.push(#table_name);
                        query_builder.push(" (");
                        query_builder.push(#insert_columns_str);
                        query_builder.push(") VALUES (");
                        query_builder.push(#insert_placeholders_str);
                        query_builder.push(")");
                        let query = query_builder.build();
                        if rust_eloquent::schema::is_query_log_enabled() {
                            println!("[SQL Debug] {}", query.sql());
                        }
                        let result = query
                            #(#bind_inserts)*
                            .execute(executor)
                            .await?;
                        self.id = result.last_insert_id().unwrap_or(0) as i32;
                    }
                    {
                        let observers = {
                            let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.created(self).await?;
                        }
                    }
                } else {
                    {
                        let observers = {
                            let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.updating(self).await?;
                        }
                    }
                    use rust_eloquent::sqlx::query_builder::QueryBuilder;
                    let mut query_builder = QueryBuilder::new("UPDATE ");
                    query_builder.push(#table_name);
                    query_builder.push(" SET ");
                    query_builder.push(#update_sets_str);
                    query_builder.push(" WHERE id = ?");
                    let query = query_builder.build();
                    if rust_eloquent::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {} | ID: {}", query.sql(), self.id);
                    }
                    query
                        #(#bind_updates)*
                        .bind(self.id)
                        .execute(executor)
                        .await?;
                    {
                        let observers = {
                            let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.updated(self).await?;
                        }
                    }
                }
                {
                    let observers = {
                        let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                        list.clone()
                    };
                    for obs in observers.iter() {
                        obs.saved(self).await?;
                    }
                }
                #[cfg(feature = "redis")]
                {
                    use rust_eloquent::redis::AsyncCommands;
                    let mut conn = rust_eloquent::Eloquent::redis_manager();
                    let payload = self.to_json();
                    if is_new {
                        let topic = format!("eloquent:events:{}:created", #table_name);
                        if let Err(e) = conn.publish(&topic, &payload).await {
                            eprintln!("[Redis Error] Failed to publish created event: {}", e);
                        }
                    } else {
                        let topic = format!("eloquent:events:{}:updated", #table_name);
                        if let Err(e) = conn.publish(&topic, &payload).await {
                            eprintln!("[Redis Error] Failed to publish updated event: {}", e);
                        }
                    }
                    let topic = format!("eloquent:events:{}:saved", #table_name);
                    if let Err(e) = conn.publish(&topic, &payload).await {
                        eprintln!("[Redis Error] Failed to publish saved event: {}", e);
                    }
                }
                #hook_after_save
                Ok(())
            }

            pub async fn delete(&self) -> Result<(), rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                self.delete_with_tx_internal(pool).await
            }

            pub async fn delete_with_tx(&self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::sqlx::Any>) -> Result<(), rust_eloquent::sqlx::Error> {
                self.delete_with_tx_internal(&mut **tx).await
            }

            async fn delete_with_tx_internal<'e, E>(&self, executor: E) -> Result<(), rust_eloquent::sqlx::Error> 
            where E: rust_eloquent::sqlx::Executor<'e, Database = rust_eloquent::sqlx::Any>
            {
                #hook_before_delete
                {
                    let observers = {
                        let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                        list.clone()
                    };
                    for obs in observers.iter() {
                        obs.deleting(self).await?;
                    }
                }
                #delete_logic
                if rust_eloquent::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {} | ID: {}", query, self.id);
                }
                rust_eloquent::sqlx::query(&query).bind(self.id).execute(executor).await?;
                {
                    let observers = {
                        let list = Self::observers().read().expect("Failed to acquire read lock on observers - possible poisoning");
                        list.clone()
                    };
                    for obs in observers.iter() {
                        obs.deleted(self).await?;
                    }
                }
                #[cfg(feature = "redis")]
                {
                    use rust_eloquent::redis::AsyncCommands;
                    let mut conn = rust_eloquent::Eloquent::redis_manager();
                    let payload = self.to_json();
                    let topic = format!("eloquent:events:{}:deleted", #table_name);
                    if let Err(e) = conn.publish(&topic, &payload).await {
                        eprintln!("[Redis Error] Failed to publish deleted event: {}", e);
                    }
                }
                #hook_after_delete
                Ok(())
            }

            pub async fn restore(&self) -> Result<(), rust_eloquent::sqlx::Error> {
                if #has_soft_deletes {
                    let pool = rust_eloquent::Eloquent::pool();
                    let query = format!("UPDATE {} SET deleted_at = NULL WHERE id = ?", #table_name);
                    rust_eloquent::sqlx::query(&query).bind(self.id).execute(pool).await?;
                }
                Ok(())
            }

            pub async fn force_delete(&self) -> Result<(), rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let query = format!("DELETE FROM {} WHERE id = ?", #table_name);
                rust_eloquent::sqlx::query(&query).bind(self.id).execute(pool).await?;
                Ok(())
            }

            pub fn to_json(&self) -> String {
                let mut map = rust_eloquent::serde_json::Map::new();
                #(#to_json_fields)*
                rust_eloquent::serde_json::Value::Object(map).to_string()
            }
        }
    }
}
