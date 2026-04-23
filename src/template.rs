pub fn replace_vars(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{key}}}}}");
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_substitution() {
        let result = replace_vars(
            "Hello {{name}}, welcome to {{project}}!",
            &[("name", "World"), ("project", "Yoke")],
        );
        assert_eq!(result, "Hello World, welcome to Yoke!");
    }

    #[test]
    fn no_vars() {
        let result = replace_vars("No variables here.", &[]);
        assert_eq!(result, "No variables here.");
    }

    #[test]
    fn missing_var_left_as_is() {
        let result = replace_vars("Hello {{name}}!", &[("other", "value")]);
        assert_eq!(result, "Hello {{name}}!");
    }

    #[test]
    fn multiple_occurrences() {
        let result = replace_vars("{{x}} and {{x}} again", &[("x", "val")]);
        assert_eq!(result, "val and val again");
    }

    #[test]
    fn empty_template() {
        let result = replace_vars("", &[("x", "y")]);
        assert_eq!(result, "");
    }

    #[test]
    fn adjacent_vars() {
        let result = replace_vars("{{a}}{{b}}", &[("a", "1"), ("b", "2")]);
        assert_eq!(result, "12");
    }
}
