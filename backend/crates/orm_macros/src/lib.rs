extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Lit, Meta};

#[proc_macro_derive(Entity, attributes(entity))]
pub fn entity_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let table_name = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("entity") {
                if let Meta::List(meta_list) = &attr.meta {
                    if let Ok(expr) = meta_list.parse_args::<syn::ExprAssign>() {
                        if let syn::Expr::Lit(expr_lit) = *expr.right {
                            if let syn::Lit::Str(lit_str) = expr_lit.lit {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
            None
        })
        .expect("`table_name` attribute is required, e.g. #[entity(table_name = \"users\")]");

    let fields = if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields) = &data.fields {
            &fields.named
        } else {
            panic!("Entity derive macro only supports structs with named fields");
        }
    } else {
        panic!("Entity derive macro only supports structs");
    };

    let mut pk_fields = Vec::new();
    let mut pk_auto_increment = false;

    for field in fields {
        for attr in &field.attrs {
            if attr.path().is_ident("entity") {
                if let Meta::List(list) = &attr.meta {
                    list.parse_nested_meta(|meta| {
                        if meta.path.is_ident("primary_key") {
                            pk_fields.push((field.ident.as_ref().unwrap().clone(), field.ty.clone()));

                            if let Ok(list_inner) = meta.input.parse::<syn::MetaList>() {
                                list_inner.parse_nested_meta(|nested| {
                                    if nested.path.is_ident("auto_increment") {
                                        if let Ok(val) = nested.value()?.parse::<Lit>() {
                                            if let Lit::Bool(b) = val {
                                                pk_auto_increment = b.value;
                                            }
                                        }
                                    }
                                    Ok(())
                                })?;
                            }
                        }
                        Ok(())
                    }).ok();
                }
            }
        }
    }

    if pk_fields.is_empty() {
        panic!("At least one `primary_key` attribute is required.");
    }

    if pk_auto_increment && pk_fields.len() > 1 {
        panic!("`auto_increment` can only be true for single-column primary keys.");
    }

    let pk_field_idents: Vec<_> = pk_fields.iter().map(|(ident, _)| ident).collect();
    let pk_field_tys: Vec<_> = pk_fields.iter().map(|(_, ty)| ty).collect();

    let (pk_type, pk_field_name_str_literal, pk_where_clause, pk_to_params_body, pk_id_param) = if pk_fields.len() == 1 {
        let pk_type = &pk_field_tys[0];
        let pk_field_name_str = pk_field_idents[0].to_string();
        let pk_field_name_str_literal = quote! { #pk_field_name_str };
        let pk_where_clause = quote! { format!("\"{}\" = ?", #pk_field_name_str) };
        let pk_field_ident = &pk_field_idents[0];
        let pk_to_params_body = quote! { vec![Box::new(self.#pk_field_ident.clone()) as Box<dyn duckdb::ToSql + Send>] };
        let pk_id_param = quote! { let params: Vec<Box<dyn duckdb::ToSql + Send>> = vec![Box::new(id)]; };
        (quote! { #pk_type }, pk_field_name_str_literal, pk_where_clause, pk_to_params_body, pk_id_param)
    } else {
        let pk_types_tuple = quote! { (#(#pk_field_tys),*) };
        let pk_field_names_str: Vec<_> = pk_field_idents.iter().map(|i| i.to_string()).collect();
        let pk_field_name_str_literal = quote! { &[#(#pk_field_names_str),*] };
        let where_clauses: Vec<_> = pk_field_names_str.iter().map(|name| format!("\"{}\" = ?", name)).collect();
        let where_clause_str = where_clauses.join(" AND ");
        let pk_where_clause = quote! { #where_clause_str.to_string() };

        let pk_to_params_stmts = pk_field_idents.iter().map(|ident| {
            quote! { Box::new(self.#ident.clone()) as Box<dyn duckdb::ToSql + Send> }
        });
        let pk_to_params_body = quote! { vec![#(#pk_to_params_stmts),*] };

        let id_destructure = pk_field_idents.iter().enumerate().map(|(i, _)| {
            let index = syn::Index::from(i);
            quote! { Box::new(id.#index.clone()) as Box<dyn duckdb::ToSql + Send> }
        });
        let pk_id_param = quote! { let params: Vec<Box<dyn duckdb::ToSql + Send>> = vec![#(#id_destructure),*]; };

        (pk_types_tuple, pk_field_name_str_literal, pk_where_clause, pk_to_params_body, pk_id_param)
    };

    let from_row_fields = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let mut is_json = false;
        for attr in &field.attrs {
            if attr.path().is_ident("entity") {
                if let Meta::List(list) = &attr.meta {
                    list.parse_nested_meta(|meta| {
                        if meta.path.is_ident("json") {
                            is_json = true;
                        }
                        Ok(())
                    }).ok();
                }
            }
        }

        if is_json {
            quote! {
                #field_name: crate::db::orm::entity::json_from_row(row, #field_name_str)?
            }
        } else {
            quote! {
                #field_name: row.get(#field_name_str)?
            }
        }
    });

    let from_row_fields_with_prefix = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let mut is_json = false;
        for attr in &field.attrs {
            if attr.path().is_ident("entity") {
                if let Meta::List(list) = &attr.meta {
                    list.parse_nested_meta(|meta| {
                        if meta.path.is_ident("json") {
                            is_json = true;
                        }
                        Ok(())
                    }).ok();
                }
            }
        }

        if is_json {
            quote! {
                #field_name: crate::db::orm::entity::json_from_row(row, format!("{}{}", prefix, #field_name_str).as_str())?
            }
        } else {
            quote! {
                #field_name: row.get(format!("{}{}", prefix, #field_name_str).as_str())?
            }
        }
    });

    let all_field_names_str = fields.iter().map(|field| {
        let field_name_str = field.ident.as_ref().unwrap().to_string();
        quote! { #field_name_str }
    });

    let column_enum_name = Ident::new("Column", struct_name.span());
    let column_variants: Vec<_> = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let variant_name = Ident::new(
            &heck::AsUpperCamelCase(&field_name.to_string()).to_string(),
            field_name.span(),
        );
        quote! { #variant_name }
    }).collect();

    let column_as_str_arms = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let variant_name = Ident::new(
            &heck::AsUpperCamelCase(&field_name.to_string()).to_string(),
            field_name.span(),
        );
        quote! {
            #column_enum_name::#variant_name => #field_name_str
        }
    });

    let insertable_fields = fields.iter().filter(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        if pk_auto_increment {
            // Only one PK is allowed if auto_increment is true
            field_ident != pk_field_idents[0]
        } else {
            // If not auto-increment, all fields are insertable
            true
        }
    });

    let to_params_fields = insertable_fields.clone().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let mut is_json = false;
        for attr in &field.attrs {
            if attr.path().is_ident("entity") {
                if let Meta::List(list) = &attr.meta {
                    list.parse_nested_meta(|meta| {
                        if meta.path.is_ident("json") {
                            is_json = true;
                        }
                        Ok(())
                    }).ok();
                }
            }
        }

        if is_json {
            quote! {
                Box::new(serde_json::to_string(&self.#field_name).ok()) as Box<dyn duckdb::ToSql + Send>
            }
        } else {
            quote! {
                Box::new(self.#field_name.clone()) as Box<dyn duckdb::ToSql + Send>
            }
        }
    });

    let insert_columns_str = insertable_fields.clone()
        .map(|f| format!("\"{}\"", f.ident.as_ref().unwrap()))
        .collect::<Vec<_>>()
        .join(", ");

    let values_str = insertable_fields.clone()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");

    let updateable_fields = fields.iter().filter(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        !pk_field_idents.contains(&field_ident)
    });

    let update_set_clause = updateable_fields.clone()
        .map(|f| format!("\"{}\" = ?", f.ident.as_ref().unwrap()))
        .collect::<Vec<_>>()
        .join(", ");

    let to_params_for_update_fields = updateable_fields.clone().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let mut is_json = false;
        for attr in &field.attrs {
            if attr.path().is_ident("entity") {
                if let Meta::List(list) = &attr.meta {
                    list.parse_nested_meta(|meta| {
                        if meta.path.is_ident("json") {
                            is_json = true;
                        }
                        Ok(())
                    }).ok();
                }
            }
        }

        if is_json {
            quote! {
                Box::new(serde_json::to_string(&self.#field_name).ok()) as Box<dyn duckdb::ToSql + Send>
            }
        } else {
            quote! {
                Box::new(self.#field_name.clone()) as Box<dyn duckdb::ToSql + Send>
            }
        }
    }).collect::<Vec<_>>();


    let expanded = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #column_enum_name {
            #(#column_variants),*
        }

        impl #column_enum_name {
            pub fn iter() -> impl Iterator<Item = Self> {
                [
                    #(Self::#column_variants),*
                ].iter().copied()
            }
        }

        impl crate::db::orm::entity::Column for #column_enum_name {
            fn as_str(&self) -> &'static str {
                match self {
                    #(#column_as_str_arms),*
                }
            }
        }

        #[async_trait::async_trait]
        impl crate::db::orm::entity::Entity for #struct_name {
            type PrimaryKey = #pk_type;
            type Column = #column_enum_name;

            const TABLE_NAME: &'static str = #table_name;

            fn from_row_with_prefix(row: &async_duckdb::row::OwnedRow, prefix: &str) -> Result<Self, duckdb::Error> {
                Ok(Self {
                    #(#from_row_fields_with_prefix),*
                })
            }

            fn columns() -> Vec<&'static str> {
                vec![#(#all_field_names_str),*]
            }

            async fn insert<E: crate::db::orm::executor::Executor + Send>(self, exec: &mut E) -> Result<Self, crate::web::error::AppError> {
                let sql = format!(
                    "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
                    Self::TABLE_NAME,
                    #insert_columns_str,
                    #values_str
                );

                let params = self.to_params();
                
                exec.execute_with_params_and_return_row(&sql, params).await
            }

            async fn update<E: crate::db::orm::executor::Executor + Send>(self, exec: &mut E) -> Result<Self, crate::web::error::AppError> {
                let sql = format!(
                    "UPDATE {} SET {} WHERE {} RETURNING *",
                    Self::TABLE_NAME,
                    #update_set_clause,
                    #pk_where_clause
                );

                let mut params = self.to_params_for_update();
                params.extend(self.pk_to_params());

                exec.execute_with_params_and_return_row(&sql, params).await
            }

            async fn find_by_id<E: crate::db::orm::executor::Executor + Send>(
                exec: &mut E,
                id: Self::PrimaryKey,
            ) -> Result<Option<Self>, crate::web::error::AppError> {
                let sql = format!("SELECT * FROM {} WHERE {} LIMIT 1", Self::TABLE_NAME, #pk_where_clause);
                #pk_id_param
                exec.fetch_optional_one(&sql, params).await
            }

            async fn delete<E: crate::db::orm::executor::Executor + Send>(exec: &mut E, id: Self::PrimaryKey) -> Result<u64, crate::web::error::AppError> {
                let sql = format!(
                    "DELETE FROM {} WHERE {}",
                    Self::TABLE_NAME,
                    #pk_where_clause
                );
                #pk_id_param
                let result = exec.execute(&sql, params).await?;
                Ok(result as u64)
            }

            async fn delete_many<E: crate::db::orm::executor::Executor + Send>(builder: crate::db::orm::query::QueryBuilder<'_, Self>, exec: &mut E) -> Result<u64, crate::web::error::AppError> {
                builder.delete(exec).await
            }
        }

        impl crate::db::orm::entity::FromRow for #struct_name {
            fn from_row(row: &async_duckdb::row::OwnedRow) -> Result<Self, duckdb::Error> {
                Ok(Self {
                    #(#from_row_fields),*
                })
            }
        }

        impl #struct_name {
            fn to_params(&self) -> Vec<Box<dyn duckdb::ToSql + Send>> {
                vec![
                    #(#to_params_fields),*
                ]
            }

            fn to_params_for_update(&self) -> Vec<Box<dyn duckdb::ToSql + Send>> {
                vec![
                    #(#to_params_for_update_fields),*
                ]
            }

            fn pk_to_params(&self) -> Vec<Box<dyn duckdb::ToSql + Send>> {
                #pk_to_params_body
            }
        }
    };

    TokenStream::from(expanded)
}
