use quote::quote;
use proc_macro2::TokenStream;
use crate::parser::ParsedModel;

pub struct GeneratedRelationships {
    pub flags: Vec<TokenStream>,
    pub inits: Vec<TokenStream>,
    pub methods: Vec<TokenStream>,
    pub model_methods: Vec<TokenStream>,
    pub eager_loads: TokenStream,
}

pub fn generate(parsed: &ParsedModel) -> GeneratedRelationships {
    let mut flags = vec![];
    let mut inits = vec![];
    let mut methods = vec![];
    let mut model_methods = vec![];

    let name = &parsed.name;

    for rel in &parsed.relations {
        let field_name = &rel.field_name;
        let rel_type = &rel.rel_type;
        let rel_model = &rel.rel_model;
        let foreign_key = &rel.foreign_key;
        let local_key = &rel.local_key;
        let related_key = &rel.related_key;
        let pivot_table = &rel.pivot_table;
        let morph_name = &rel.morph_name;

        let load_flag_ident = quote::format_ident!("load_{}", field_name);
        let filter_flag_ident = quote::format_ident!("filter_{}", field_name);
        let rel_model_builder_ident = quote::format_ident!("{}QueryBuilder", rel_model);

        flags.push(quote! {
            pub #load_flag_ident: bool,
            pub #filter_flag_ident: Option<std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>>,
        });
        inits.push(quote! {
            #load_flag_ident: false,
            #filter_flag_ident: None,
        });

        let with_method_ident = quote::format_ident!("with_{}", field_name);
        let with_constrained_method_ident = quote::format_ident!("with_{}_constrained", field_name);
        methods.push(quote! {
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
            model_methods.push(quote! {
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
            model_methods.push(quote! {
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
            model_methods.push(quote! {
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
            model_methods.push(quote! {
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
            model_methods.push(quote! {
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
            let _pivot_rk = format!("{}.{}", pivot_table, related_key);
            model_methods.push(quote! {
                pub fn #method_name(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let related_pk = format!("{}.{}", #rel_model_ident::table_name(), "id");
                        let select_raw = format!("{}.*", #rel_model_ident::table_name());
                        #rel_model_ident::query()
                            .select_raw(&select_raw)
                            .join(#pivot_table, &related_pk, "=", &_pivot_rk)
                            .where_eq(&#pivot_fk, self.#lk_ident.clone())
                            .get().await
                    })
                }
                pub fn #method_name_constrained(&self, modifier: std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<#rel_model_ident>, rust_eloquent::sqlx::Error>> + Send + '_>> {
                    Box::pin(async move {
                        let related_pk = format!("{}.{}", #rel_model_ident::table_name(), "id");
                        let select_raw = format!("{}.*", #rel_model_ident::table_name());
                        let mut q = #rel_model_ident::query()
                            .select_raw(&select_raw)
                            .join(#pivot_table, &related_pk, "=", &_pivot_rk)
                            .where_eq(&#pivot_fk, self.#lk_ident.clone());
                        q = modifier(q);
                        q.get().await
                    })
                }
            });
        }
    }

    let eager_loads_logic: Vec<_> = parsed.relations.iter().map(|rel| {
        let field_name = &rel.field_name;
        let rel_type = &rel.rel_type;
        let rel_model = &rel.rel_model;
        let foreign_key = &rel.foreign_key;
        let local_key = &rel.local_key;
        let related_key = &rel.related_key;

        let load_flag = quote::format_ident!("load_{}", field_name);
        let filter_flag = quote::format_ident!("filter_{}", field_name);
        let method_name = quote::format_ident!("{}", field_name);
        
        let rel_model_ident = syn::Ident::new(rel_model, field_name.span());
        let fk_ident = quote::format_ident!("{}", if foreign_key.is_empty() { format!("{}_id", name.to_string().to_lowercase()) } else { foreign_key.clone() });
        let lk_ident = quote::format_ident!("{}", if local_key.is_empty() { "id".to_string() } else { local_key.clone() });
        let pk_ident = quote::format_ident!("{}", if related_key.is_empty() { "id".to_string() } else { related_key.clone() });

        if rel_type == "has_many" {
            quote! {
                if self.#load_flag {
                    let parent_ids: Vec<_> = results.iter().map(|m| m.#lk_ident.clone()).collect();
                    if !parent_ids.is_empty() {
                        let mut query = #rel_model_ident::query().where_in(stringify!(#fk_ident), parent_ids);
                        if let Some(ref filter) = self.#filter_flag {
                            query = filter(query);
                        }
                        let mut all_related = Box::pin(query.get()).await?;
                        
                        for model in &mut results {
                            let mut matching = vec![];
                            let mut i = 0;
                            while i < all_related.len() {
                                if all_related[i].#fk_ident == model.#lk_ident {
                                    matching.push(all_related.remove(i));
                                } else {
                                    i += 1;
                                }
                            }
                            model.#method_name = Some(matching);
                        }
                    }
                }
            }
        } else if rel_type == "has_one" {
            quote! {
                if self.#load_flag {
                    let parent_ids: Vec<_> = results.iter().map(|m| m.#lk_ident.clone()).collect();
                    if !parent_ids.is_empty() {
                        let mut query = #rel_model_ident::query().where_in(stringify!(#fk_ident), parent_ids);
                        if let Some(ref filter) = self.#filter_flag {
                            query = filter(query);
                        }
                        let mut all_related = Box::pin(query.get()).await?;
                        
                        for model in &mut results {
                            let mut matching = None;
                            let mut i = 0;
                            while i < all_related.len() {
                                if all_related[i].#fk_ident == model.#lk_ident {
                                    matching = Some(all_related.remove(i));
                                    break;
                                }
                                i += 1;
                            }
                            model.#method_name = matching;
                        }
                    }
                }
            }
        } else if rel_type == "belongs_to" {
            quote! {
                if self.#load_flag {
                    let parent_ids: Vec<_> = results.iter().map(|m| m.#fk_ident.clone()).collect();
                    if !parent_ids.is_empty() {
                        let mut query = #rel_model_ident::query().where_in(stringify!(#pk_ident), parent_ids);
                        if let Some(ref filter) = self.#filter_flag {
                            query = filter(query);
                        }
                        let mut all_related = Box::pin(query.get()).await?;
                        
                        for model in &mut results {
                            let mut matching = None;
                            let mut i = 0;
                            while i < all_related.len() {
                                if all_related[i].#pk_ident == model.#fk_ident {
                                    matching = Some(all_related.remove(i));
                                    break;
                                }
                                i += 1;
                            }
                            model.#method_name = matching;
                        }
                    }
                }
            }
        } else {
            let method_name_constrained = quote::format_ident!("{}_constrained", field_name);
            if rel_type == "morph_many" || rel_type == "belongs_to_many" {
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
        }
    }).collect();

    GeneratedRelationships {
        flags,
        inits,
        methods,
        model_methods,
        eager_loads: quote! { #(#eager_loads_logic)* },
    }
}
