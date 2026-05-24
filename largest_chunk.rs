                        for model in results.iter_mut() {
                            let matching = related.iter().find(|rel| rel.#morph_id_ident == model.#lk_ident).cloned();
                            model.#method_name = matching;
                        }
                    }
                });
            } else if rel_type == "belongs_to_many" {
                eager_loads.push(quote! {
                    if self.#load_flag_ident && !results.is_empty() {
                        let mut futures = vec![];
                        for model in &results {
                            futures.push(model.#method_name());
                        }
                        let resolved_rels = rust_eloquent::futures::future::join_all(futures).await;
                        for (i, result) in resolved_rels.into_iter().enumerate() {
                            if let Ok(rel) = result {
                                results[i].#method_name = Some(rel);
                            }
                        }
                    }
                });
            }

            continue;