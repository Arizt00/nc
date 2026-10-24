
// ===== BIND VALIDATION =====

fn validate_bind_section(container: &Container) -> Result<(), SemanticError> {
    let mut stc_symbols: HashSet<String> = HashSet::new();

    for decl in &container.stc {
        stc_symbols.insert(decl.name.clone());
    }

    for bind in &container.bind {
        let target = &bind.target;
        let value = &bind.value;

        if !stc_symbols.contains(&target.name) {
            return Err(SemanticError::new(
                bind.span,
                format!("bind target '{}' not declared in stc", target.name),
            ));
        }

        if !is_base_pointer_type(&bind.ty) {
            return Err(SemanticError::new(
                bind.span,
                format!("bind only allows base pointer/address types, got {:?}", bind.ty),
            ));
        }

        if let Expr::Ref(ref_name) = value {
            if !stc_symbols.contains(ref_name) {
                return Err(SemanticError::new(
                    bind.span,
                    format!("bind source '{}' not declared in stc", ref_name),
                ));
            }
        }
    }

    Ok(())
}

// ===== END BIND VALIDATION =====

