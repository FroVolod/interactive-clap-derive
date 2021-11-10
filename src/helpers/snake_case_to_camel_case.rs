pub fn snake_case_to_camel_case(s: String) -> String {
    let s_vec: Vec<String> = s.split('_')
        .map(|s| s.replace(s.chars().next().unwrap(), &s[..1].to_ascii_uppercase()))
        .collect();
    s_vec.join("")
}
