extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(Eloquent, attributes(eloquent))]
pub fn eloquent_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let mut table_name = format!("{}s", name.to_string().to_lowercase());
    let mut global_scope = String::new();
    let mut before_save = String::new();
    let mut after_save = String::new();
    let mut before_delete = String::new();
    let mut after_delete = String::new();
    let mut after_fetch = String::new();

    for attr in &input.attrs {
        if attr.path().is_ident("eloquent") {
            let token_str = attr.meta.require_list().unwrap().tokens.to_string();
            for part in token_str.split(',') {
                let parts: Vec<&str> = part.split('=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let val = parts[1].trim().trim_matches('"');
                    match key {
                        "table" => table_name = val.to_string(),
                        "global_scope" => global_scope = val.to_string(),
                        "before_save" => before_save = val.to_string(),
                        "after_save" => after_save = val.to_string(),
                        "before_delete" => before_delete = val.to_string(),
                        "after_delete" => after_delete = val.to_string(),
                        "after_fetch" => after_fetch = val.to_string(),
                        _ => {}
                    }
                }
            }
        }
    }

    let builder_name_str = format!("{}QueryBuilder", name);
    let builder_name = syn::Ident::new(&builder_name_str, name.span());
    let column_enum_name_str = format!("{}Column", name);
    let column_enum_name = syn::Ident::new(&column_enum_name_str, name.span());
    let factory_name_str = format!("{}Factory", name);
    let factory_name = syn::Ident::new(&factory_name_str, name.span());

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("Eloquent macro only supports structs with named fields"),
        },
        _ => panic!("Eloquent macro can only be used on structs"),
    };

    let mut normal_fields = vec![];
    let mut hidden_fields = vec![];
    let mut relations = vec![];
    let mut has_soft_deletes = false;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        if field_name_str == "deleted_at" { has_soft_deletes = true; }
        
        let mut is_relation = false;
        let mut rel_type = String::new();
        let mut rel_model = String::new();
        let mut foreign_key = String::new();
        let mut related_key = String::new();
        let mut pivot_table = String::new();
        let mut local_key = "id".to_string();
        let mut morph_name = String::new();
        let mut is_hidden = false;

        for attr in &field.attrs {
            if attr.path().is_ident("eloquent") {
                let token_str = attr.meta.require_list().unwrap().tokens.to_string();
                for part in token_str.split(',') {
                    let trimmed = part.trim();
                    if trimmed == "hidden" {
                        is_hidden = true;
                    } else {
                        let parts: Vec<&str> = trimmed.split('=').collect();
                        if parts.len() == 2 {
                            let key = parts[0].trim();
                            let val = parts[1].trim().trim_matches('"');
                            match key {
                                "has_many" => { is_relation = true; rel_type = "has_many".to_string(); rel_model = val.to_string(); }
                                "has_one" => { is_relation = true; rel_type = "has_one".to_string(); rel_model = val.to_string(); }
                                "belongs_to" => { is_relation = true; rel_type = "belongs_to".to_string(); rel_model = val.to_string(); }
                                "belongs_to_many" => { is_relation = true; rel_type = "belongs_to_many".to_string(); rel_model = val.to_string(); }
                                "morph_many" => { is_relation = true; rel_type = "morph_many".to_string(); rel_model = val.to_string(); }
                                "morph_one" => { is_relation = true; rel_type = "morph_one".to_string(); rel_model = val.to_string(); }
                                "foreign_key" => foreign_key = val.to_string(),
                                "related_key" => related_key = val.to_string(),
                                "pivot_table" => pivot_table = val.to_string(),
                                "local_key" => local_key = val.to_string(),
                                "name" => morph_name = val.to_string(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if is_relation {
            relations.push((field_name, rel_type, rel_model, foreign_key, local_key, related_key, pivot_table, morph_name));
        } else {
            normal_fields.push(field_name);
            if is_hidden {
                hidden_fields.push(field_name);
            }
        }
    }

    let mut insert_columns_vec = vec![];
    let mut insert_placeholders_vec = vec![];
    let mut update_sets_vec = vec![];
    let mut bind_inserts = vec![];
    let mut bind_updates = vec![];

    for ident in &normal_fields {
        let name = ident.to_string();
        if name == "id" { continue; }
        if name == "created_at" {
            insert_columns_vec.push(name.clone());
            insert_placeholders_vec.push("CURRENT_TIMESTAMP".to_string());
            continue;
        }
        if name == "updated_at" {
            insert_columns_vec.push(name.clone());
            insert_placeholders_vec.push("CURRENT_TIMESTAMP".to_string());
            update_sets_vec.push(format!("{} = CURRENT_TIMESTAMP", name));
            continue;
        }
        insert_columns_vec.push(name.clone());
        insert_placeholders_vec.push("?".to_string());
        bind_inserts.push(quote! { .bind(self.#ident.clone()) });
        update_sets_vec.push(format!("{} = ?", name));
        bind_updates.push(quote! { .bind(self.#ident.clone()) });
    }

    let insert_columns = insert_columns_vec.join(", ");
    let insert_placeholders = insert_placeholders_vec.join(", ");
    let update_sets = update_sets_vec.join(", ");

    let delete_logic = if has_soft_deletes {
        quote! { let query = format!("UPDATE {} SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?", #table_name); }
    } else {
        quote! { let query = format!("DELETE FROM {} WHERE id = ?", #table_name); }
    };
    let delete_all_logic = if has_soft_deletes {
        quote! { let mut query_str = format!("UPDATE {} SET deleted_at = CURRENT_TIMESTAMP", #table_name); }
    } else {
        quote! { let mut query_str = format!("DELETE FROM {}", #table_name); }
    };

    // ==========================================
    // MAGIC METHODS GENERATION
    // ==========================================
    let magic_methods: Vec<_> = normal_fields.iter().map(|ident| {
        let field_name_str = ident.to_string();
        let where_method = quote::format_ident!("where_{}", ident);
        let where_not_method = quote::format_ident!("where_not_{}", ident);
        let order_by_method = quote::format_ident!("order_by_{}", ident);
        let order_by_desc_method = quote::format_ident!("order_by_{}_desc", ident);

        quote! {
            pub fn #where_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.where_eq(#field_name_str, value)
            }
            pub fn #where_not_method<T: Into<rust_eloquent::EloquentValue>>(self, value: T) -> Self {
                self.where_not_eq(#field_name_str, value)
            }
            pub fn #order_by_method(self) -> Self {
                self.order_by(#field_name_str)
            }
            pub fn #order_by_desc_method(self) -> Self {
                self.order_by_desc(#field_name_str)
            }
        }
    }).collect();

    // ==========================================
    // COLUMN ENUM GENERATION
    // ==========================================
    let column_variants: Vec<_> = normal_fields.iter().map(|ident| {
        let name = ident.to_string();
        let mut chars = name.chars();
        let mut camel = match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        };
        // Quick snake_case to PascalCase (just to make it valid Rust enum)
        camel = camel.split('_').map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }).collect();
        let variant = syn::Ident::new(&camel, ident.span());
        quote! { #variant }
    }).collect();

    let column_to_string: Vec<_> = normal_fields.iter().zip(column_variants.iter()).map(|(ident, variant)| {
        let field_name_str = ident.to_string();
        quote! { #column_enum_name::#variant => #field_name_str }
    }).collect();

    let enum_def = quote! {
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

    let to_json_fields: Vec<_> = normal_fields.iter()
        .filter(|ident| !hidden_fields.iter().any(|h| h.to_string() == ident.to_string()))
        .map(|ident| {
            let field_name_str = ident.to_string();
            quote! {
                map.insert(#field_name_str.to_string(), rust_eloquent::serde_json::json!(self.#ident));
            }
        })
        .collect();

    // ==========================================
    // BUILDER EXTENSIONS (Relationships, Scopes)
    // ==========================================
    let mut relation_flags = vec![];
    let mut relation_inits = vec![];
    let mut relation_methods = vec![];
    let mut relationship_methods = vec![];

    for (field_name, rel_type, rel_model, foreign_key, local_key, related_key, pivot_table, morph_name) in &relations {
        let load_flag_ident = quote::format_ident!("load_{}", field_name);
        let filter_flag_ident = quote::format_ident!("filter_{}", field_name);
        let rel_model_builder_ident = quote::format_ident!("{}QueryBuilder", rel_model);

        relation_flags.push(quote! {
            pub #load_flag_ident: bool,
            pub #filter_flag_ident: Option<std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>>,
        });
        relation_inits.push(quote! {
            #load_flag_ident: false,
            #filter_flag_ident: None,
        });

        let with_method_ident = quote::format_ident!("with_{}", field_name);
        let with_constrained_method_ident = quote::format_ident!("with_{}_constrained", field_name);
        relation_methods.push(quote! {
            pub fn #with_method_ident(mut self) -> Self {
                self.#load_flag_ident = true;
                self
            }
            pub fn #with_constrained_method_ident<F>(mut self, filter: F) -> Self
            where F: Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync + 'static {
                self.#load_flag_ident = true;
                self.#filter_flag_ident = Some(std::sync::Arc::new(filter));
                self
            }
        });

        let rel_model_ident = syn::Ident::new(rel_model, field_name.span());
        let method_name = quote::format_ident!("{}", field_name);
        let method_name_constrained = quote::format_ident!("{}_constrained", field_name);
        let fk_ident = quote::format_ident!("{}", if foreign_key.is_empty() { format!("{}_id", name.to_string().to_lowercase()) } else { foreign_key.clone() });
        let lk_ident = quote::format_ident!("{}", if local_key.is_empty() { "id".to_string() } else { local_key.clone() });
        let pk_ident = quote::format_ident!("{}", if related_key.is_empty() { "id".to_string() } else { related_key.clone() });

        if rel_type == "has_many" {
            relationship_methods.push(quote! {
                pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone()).get().await
                    })
                }
                pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let mut q = #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone());
                        q = modifier(q);
                        q.get().await
                    })
                }
            });
        } else if rel_type == "has_one" {
            relationship_methods.push(quote! {
                pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone()).first().await
                    })
                }
                pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let mut q = #rel_model_ident::query().where_eq(stringify!(#fk_ident), self.#lk_ident.clone());
                        q = modifier(q);
                        q.first().await
                    })
                }
            });
        } else if rel_type == "belongs_to" {
            relationship_methods.push(quote! {
                pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        #rel_model_ident::query().where_eq(stringify!(#pk_ident), self.#fk_ident.clone()).first().await
                    })
                }
                pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let mut q = #rel_model_ident::query().where_eq(stringify!(#pk_ident), self.#fk_ident.clone());
                        q = modifier(q);
                        q.first().await
                    })
                }
            });
        } else if rel_type == "morph_many" {
            let morph_type_ident = quote::format_ident!("{}_type", morph_name);
            let morph_id_ident = quote::format_ident!("{}_id", morph_name);
            relationship_methods.push(quote! {
                pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        #rel_model_ident::query()
                            .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                            .where_eq(stringify!(#morph_type_ident), stringify!(#name))
                            .get().await
                    })
                }
                pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let mut q = #rel_model_ident::query()
                            .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                            .where_eq(stringify!(#morph_type_ident), stringify!(#name));
                        q = modifier(q);
                        q.get().await
                    })
                }
            });
        } else if rel_type == "morph_one" {
            let morph_type_ident = quote::format_ident!("{}_type", morph_name);
            let morph_id_ident = quote::format_ident!("{}_id", morph_name);
            relationship_methods.push(quote! {
                pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        #rel_model_ident::query()
                            .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                            .where_eq(stringify!(#morph_type_ident), stringify!(#name))
                            .first().await
                    })
                }
                pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let mut q = #rel_model_ident::query()
                            .where_eq(stringify!(#morph_id_ident), self.#lk_ident.clone())
                            .where_eq(stringify!(#morph_type_ident), stringify!(#name));
                        q = modifier(q);
                        q.first().await
                    })
                }
            });
        } else if rel_type == "belongs_to_many" {
            let pivot_fk = format!("{}.{}", pivot_table, foreign_key);
            let pivot_rk = format!("{}.{}", pivot_table, related_key);
            relationship_methods.push(quote! {
                pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let related_pk = format!("{}.{}", #rel_model_ident::table_name(), "id");
                        #rel_model_ident::query()
                            .select_raw(&format!("{}.*", #rel_model_ident::table_name()))
                            .join(#pivot_table, &related_pk, "=", #pivot_rk)
                            .where_eq(&#pivot_fk, self.#lk_ident.clone())
                            .get().await
                    })
                }
                pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let related_pk = format!("{}.{}", #rel_model_ident::table_name(), "id");
                        let mut q = #rel_model_ident::query()
                            .select_raw(&format!("{}.*", #rel_model_ident::table_name()))
                            .join(#pivot_table, &related_pk, "=", #pivot_rk)
                            .where_eq(&#pivot_fk, self.#lk_ident.clone());
                        q = modifier(q);
                        q.get().await
                    })
                }
            });
        }
    }

    let eager_loads_logic: Vec<_> = relations.iter().map(|(field_name, rel_type, _, _, _, _, _, _)| {
        let load_flag = quote::format_ident!("load_{}", field_name);
        let filter_flag = quote::format_ident!("filter_{}", field_name);
        let method_name = quote::format_ident!("{}", field_name);
        let method_name_constrained = quote::format_ident!("{}_constrained", field_name);
        if rel_type == "has_many" || rel_type == "morph_many" || rel_type == "belongs_to_many" {
            quote! {
                if self.#load_flag {
                    for model in &mut results {
                        if let Some(ref filter) = self.#filter_flag {
                            model.#method_name = Some(model.#method_name_constrained(filter.clone()).await?);
                        } else {
                            model.#method_name = Some(model.#method_name().await?);
                        }
                    }
                }
            }
        } else {
            quote! {
                if self.#load_flag {
                    for model in &mut results {
                        if let Some(ref filter) = self.#filter_flag {
                            model.#method_name = model.#method_name_constrained(filter.clone()).await?;
                        } else {
                            model.#method_name = model.#method_name().await?;
                        }
                    }
                }
            }
        }
    }).collect();
    let eager_loads = quote! { #(#eager_loads_logic)* };

    let hook_before_save = if !before_save.is_empty() { let method = syn::Ident::new(&before_save, name.span()); quote! { self.#method().await?; } } else { quote! {} };
    let hook_after_save = if !after_save.is_empty() { let method = syn::Ident::new(&after_save, name.span()); quote! { self.#method().await?; } } else { quote! {} };
    let hook_before_delete = if !before_delete.is_empty() { let method = syn::Ident::new(&before_delete, name.span()); quote! { self.#method().await?; } } else { quote! {} };
    let hook_after_delete = if !after_delete.is_empty() { let method = syn::Ident::new(&after_delete, name.span()); quote! { self.#method().await?; } } else { quote! {} };
    let hook_after_fetch = if !after_fetch.is_empty() { let method = syn::Ident::new(&after_fetch, name.span()); quote! { for model in &mut results { model.#method().await?; } } } else { quote! {} };

    let global_scope_logic = if !global_scope.is_empty() {
        let method = syn::Ident::new(&global_scope, name.span());
        quote! { builder = builder.#method(); }
    } else {
        quote! {}
    };

    let factory_logic = quote! {
        pub struct #factory_name {
            generator: Box<dyn Fn() -> #name + Send + Sync>,
            count: usize,
        }
        impl #factory_name {
            pub fn count(mut self, count: usize) -> Self {
                self.count = count;
                self
            }
            pub async fn create(&self) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> {
                let mut results = vec![];
                for _ in 0..self.count {
                    let mut model = (self.generator)();
                    model.save().await?;
                    results.push(model);
                }
                Ok(results)
            }
        }
        impl #name {
            pub fn factory<F: 'static + Send + Sync + Fn() -> #name>(generator: F) -> #factory_name {
                #factory_name {
                    generator: Box::new(generator),
                    count: 1,
                }
            }
        }
    };

    let observer_trait_name = quote::format_ident!("{}Observer", name);
    let observer_trait_def = quote! {
        #[rust_eloquent::async_trait]
        pub trait #observer_trait_name: Send + Sync {
            async fn saving(&self, _model: &mut #name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
            async fn saved(&self, _model: &#name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
            async fn creating(&self, _model: &mut #name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
            async fn created(&self, _model: &#name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
            async fn updating(&self, _model: &mut #name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
            async fn updated(&self, _model: &#name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
            async fn deleting(&self, _model: &#name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
            async fn deleted(&self, _model: &#name) -> Result<(), rust_eloquent::sqlx::Error> { Ok(()) }
        }
    };

    let expanded = quote! {
        #enum_def
        #factory_logic
        #observer_trait_def

        #[rust_eloquent::async_trait]
        impl rust_eloquent::EloquentModel for #name {
            fn table_name() -> &'static str {
                #table_name
            }
        }

        impl rust_eloquent::schema::SubqueryBuilder for #builder_name {
            fn to_sql(&self) -> String {
                self.to_sql()
            }
            fn bindings(&self) -> &Vec<rust_eloquent::EloquentValue> {
                &self.bindings
            }
        }

        // ==========================================
        // QUERY BUILDER
        // ==========================================
        #[derive(Clone)]
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
            pub with_trashed: bool,
            pub only_trashed: bool,
            #(#relation_flags)*
        }

        impl #builder_name {
            pub fn new() -> Self {
                let mut builder = Self {
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
                    with_trashed: false,
                    only_trashed: false,
                    #(#relation_inits)*
                };
                #global_scope_logic
                builder
            }

            // --- Soft Deletes ---
            pub fn with_trashed(mut self) -> Self {
                self.with_trashed = true;
                self
            }
            pub fn only_trashed(mut self) -> Self {
                self.only_trashed = true;
                self
            }

            // --- Column Enum Typesafe methods ---
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

            #(#relation_methods)*

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
            pub fn join_constrained<F>(mut self, table: &str, modifier: F) -> Self
            where F: FnOnce(&mut rust_eloquent::schema::JoinClause) -> &mut rust_eloquent::schema::JoinClause {
                let mut join_clause = rust_eloquent::schema::JoinClause::new(table);
                modifier(&mut join_clause);
                let sql = format!("JOIN {} ON {}", table, join_clause.conditions.join(" AND "));
                self.joins.push(sql);
                for binding in join_clause.bindings {
                    self.bindings.push(binding);
                }
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
            pub fn where_exists<B: rust_eloquent::schema::SubqueryBuilder>(mut self, sub: B) -> Self {
                self.wheres.push(("AND".to_string(), format!("EXISTS ({})", sub.to_sql())));
                for binding in sub.bindings() {
                    self.bindings.push(binding.clone());
                }
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
                let mut sql = String::new();
                
                let select_clause = match &self.selects {
                    Some(s) => s.clone(),
                    None => "*".to_string(),
                };
                let distinct = if self.is_distinct { "DISTINCT " } else { "" };
                
                sql.push_str(&format!("SELECT {}{} FROM {}", distinct, select_clause, #table_name));

                for join in &self.joins {
                    sql.push_str(&format!(" {}", join));
                }

                let mut first_where = true;
                if !self.wheres.is_empty() {
                    sql.push_str(" WHERE ");
                    for (op, cond) in &self.wheres {
                        if first_where {
                            sql.push_str(&format!("({})", cond));
                            first_where = false;
                        } else {
                            sql.push_str(&format!(" {} ({})", op, cond));
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
                    sql.push_str(&format!(" GROUP BY {}", group));
                }

                let mut first_having = true;
                if !self.havings.is_empty() {
                    sql.push_str(" HAVING ");
                    for (op, cond) in &self.havings {
                        if first_having {
                            sql.push_str(&format!("({})", cond));
                            first_having = false;
                        } else {
                            sql.push_str(&format!(" {} ({})", op, cond));
                        }
                    }
                }

                if let Some(order) = &self.order_by {
                    sql.push_str(&format!(" ORDER BY {}", order));
                }

                if let Some(limit) = self.limit {
                    sql.push_str(&format!(" LIMIT {}", limit));
                }
                if let Some(offset) = self.offset {
                    sql.push_str(&format!(" OFFSET {}", offset));
                }

                sql
            }

            // --- Execution ---
            pub async fn get(&self) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                self.get_with_tx_internal(pool).await
            }

            pub async fn get_with_tx(&self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::sqlx::Any>) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> {
                self.get_with_tx_internal(&mut **tx).await
            }

            async fn get_with_tx_internal<'e, E>(&self, executor: E) -> Result<Vec<#name>, rust_eloquent::sqlx::Error> 
            where E: rust_eloquent::sqlx::Executor<'e, Database = rust_eloquent::sqlx::Any>
            {
                let query_str = self.to_sql();
                if rust_eloquent::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {} | Bindings: {:?}", query_str, self.bindings);
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

                let mut results: Vec<#name> = rust_eloquent::sqlx::query_as_with(&query_str, args).fetch_all(executor).await?;
                
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

            pub async fn first_with_tx(&self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::sqlx::Any>) -> Result<Option<#name>, rust_eloquent::sqlx::Error> {
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
                    println!("[SQL Debug] {} | Bindings: {:?}", query_str, total_builder.bindings);
                }
                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &total_builder.bindings {
                    match binding {
                        rust_eloquent::EloquentValue::String(s) => rust_eloquent::sqlx::Arguments::add(&mut args, s.clone()).unwrap(),
                        rust_eloquent::EloquentValue::Int(i) => rust_eloquent::sqlx::Arguments::add(&mut args, *i).unwrap(),
                        rust_eloquent::EloquentValue::Float(f) => rust_eloquent::sqlx::Arguments::add(&mut args, *f).unwrap(),
                        rust_eloquent::EloquentValue::Bool(b) => rust_eloquent::sqlx::Arguments::add(&mut args, *b).unwrap(),
                    }
                }
                
                let pool = rust_eloquent::Eloquent::pool();
                let total_row: (i64,) = rust_eloquent::sqlx::query_as_with(&query_str, args).fetch_one(pool).await?;
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
                let pool = rust_eloquent::Eloquent::pool();
                let mut builder = self.clone();
                builder.selects = Some("COUNT(*)".to_string());
                builder.limit = None;
                builder.offset = None;
                builder.order_by = None;
                let query_str = builder.to_sql();
                if rust_eloquent::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {} | Bindings: {:?}", query_str, builder.bindings);
                }
                
                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &builder.bindings {
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
                self.delete_all_with_tx_internal(pool).await
            }

            pub async fn delete_all_with_tx(&self, tx: &mut rust_eloquent::sqlx::Transaction<'static, rust_eloquent::sqlx::Any>) -> Result<u64, rust_eloquent::sqlx::Error> {
                self.delete_all_with_tx_internal(&mut **tx).await
            }

            async fn delete_all_with_tx_internal<'e, E>(&self, executor: E) -> Result<u64, rust_eloquent::sqlx::Error> 
            where E: rust_eloquent::sqlx::Executor<'e, Database = rust_eloquent::sqlx::Any>
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

                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &self.bindings {
                    match binding {
                        rust_eloquent::EloquentValue::String(s) => rust_eloquent::sqlx::Arguments::add(&mut args, s.clone()).unwrap(),
                        rust_eloquent::EloquentValue::Int(i) => rust_eloquent::sqlx::Arguments::add(&mut args, *i).unwrap(),
                        rust_eloquent::EloquentValue::Float(f) => rust_eloquent::sqlx::Arguments::add(&mut args, *f).unwrap(),
                        rust_eloquent::EloquentValue::Bool(b) => rust_eloquent::sqlx::Arguments::add(&mut args, *b).unwrap(),
                    }
                }

                if rust_eloquent::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {} | Bindings: {:?}", query_str, self.bindings);
                }
                let result = rust_eloquent::sqlx::query_with(&query_str, args).execute(executor).await?;
                Ok(result.rows_affected())
            }

            pub async fn pluck_string(&self, column: &str) -> Result<Vec<String>, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let mut builder = self.clone();
                builder.selects = Some(column.to_string());
                let query_str = builder.to_sql();
                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &builder.bindings {
                    match binding {
                        rust_eloquent::EloquentValue::String(s) => rust_eloquent::sqlx::Arguments::add(&mut args, s.clone()).unwrap(),
                        rust_eloquent::EloquentValue::Int(i) => rust_eloquent::sqlx::Arguments::add(&mut args, *i).unwrap(),
                        rust_eloquent::EloquentValue::Float(f) => rust_eloquent::sqlx::Arguments::add(&mut args, *f).unwrap(),
                        rust_eloquent::EloquentValue::Bool(b) => rust_eloquent::sqlx::Arguments::add(&mut args, *b).unwrap(),
                    }
                }
                let rows: Vec<(String,)> = rust_eloquent::sqlx::query_as_with(&query_str, args).fetch_all(pool).await?;
                Ok(rows.into_iter().map(|(s,)| s).collect())
            }

            pub async fn pluck_i32(&self, column: &str) -> Result<Vec<i32>, rust_eloquent::sqlx::Error> {
                let pool = rust_eloquent::Eloquent::pool();
                let mut builder = self.clone();
                builder.selects = Some(column.to_string());
                let query_str = builder.to_sql();
                let mut args = rust_eloquent::sqlx::any::AnyArguments::default();
                for binding in &builder.bindings {
                    match binding {
                        rust_eloquent::EloquentValue::String(s) => rust_eloquent::sqlx::Arguments::add(&mut args, s.clone()).unwrap(),
                        rust_eloquent::EloquentValue::Int(i) => rust_eloquent::sqlx::Arguments::add(&mut args, *i).unwrap(),
                        rust_eloquent::EloquentValue::Float(f) => rust_eloquent::sqlx::Arguments::add(&mut args, *f).unwrap(),
                        rust_eloquent::EloquentValue::Bool(b) => rust_eloquent::sqlx::Arguments::add(&mut args, *b).unwrap(),
                    }
                }
                let rows: Vec<(i32,)> = rust_eloquent::sqlx::query_as_with(&query_str, args).fetch_all(pool).await?;
                Ok(rows.into_iter().map(|(s,)| s).collect())
            }

            // --- Magic Dynamic Methods ---
            #(#magic_methods)*
        }

        // ==========================================
        // ACTIVE RECORD METHODS
        // ==========================================
        impl #name {
            #(#relationship_methods)*

            pub fn observe(observer: std::sync::Arc<dyn #observer_trait_name + Send + Sync>) {
                let list = Self::observers();
                let mut writer = list.write().unwrap();
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
                #hook_before_save
                {
                    let observers = {
                        let list = Self::observers().read().unwrap();
                        list.clone()
                    };
                    for obs in observers.iter() {
                        obs.saving(self).await?;
                    }
                }
                if self.id == 0 {
                    {
                        let observers = {
                            let list = Self::observers().read().unwrap();
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.creating(self).await?;
                        }
                    }
                    let driver = rust_eloquent::Eloquent::driver();
                    if driver == "postgres" {
                        let query = format!("INSERT INTO {} ({}) VALUES ({}) RETURNING id", #table_name, #insert_columns, #insert_placeholders);
                        if rust_eloquent::schema::is_query_log_enabled() {
                            println!("[SQL Debug] {}", query);
                        }
                        let row = rust_eloquent::sqlx::query(&query)
                            #(#bind_inserts)*
                            .fetch_one(executor)
                            .await?;
                        self.id = rust_eloquent::sqlx::Row::try_get(&row, "id")?;
                    } else {
                        let query = format!("INSERT INTO {} ({}) VALUES ({})", #table_name, #insert_columns, #insert_placeholders);
                        if rust_eloquent::schema::is_query_log_enabled() {
                            println!("[SQL Debug] {}", query);
                        }
                        let result = rust_eloquent::sqlx::query(&query)
                            #(#bind_inserts)*
                            .execute(executor)
                            .await?;
                        self.id = result.last_insert_id().unwrap_or(0) as i32;
                    }
                    {
                        let observers = {
                            let list = Self::observers().read().unwrap();
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.created(self).await?;
                        }
                    }
                } else {
                    {
                        let observers = {
                            let list = Self::observers().read().unwrap();
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.updating(self).await?;
                        }
                    }
                    let query = format!("UPDATE {} SET {} WHERE id = ?", #table_name, #update_sets);
                    if rust_eloquent::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {} | ID: {}", query, self.id);
                    }
                    rust_eloquent::sqlx::query(&query)
                        #(#bind_updates)*
                        .bind(self.id)
                        .execute(executor)
                        .await?;
                    {
                        let observers = {
                            let list = Self::observers().read().unwrap();
                            list.clone()
                        };
                        for obs in observers.iter() {
                            obs.updated(self).await?;
                        }
                    }
                }
                {
                    let observers = {
                        let list = Self::observers().read().unwrap();
                        list.clone()
                    };
                    for obs in observers.iter() {
                        obs.saved(self).await?;
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
                        let list = Self::observers().read().unwrap();
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
                        let list = Self::observers().read().unwrap();
                        list.clone()
                    };
                    for obs in observers.iter() {
                        obs.deleted(self).await?;
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
    };

    TokenStream::from(expanded)
}
