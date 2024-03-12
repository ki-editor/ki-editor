use convert_case::Casing;

use crate::soft_wrap::soft_wrap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transformation {
    Case(convert_case::Case),
    Join,
    Wrap,
}
impl Transformation {
    pub fn apply(&self, string: String) -> String {
        match self {
            Transformation::Case(case) => string.to_case(case.clone()),
            Transformation::Join => regex::Regex::new(r"\s*\n+\s*")
                .unwrap()
                .replace_all(&string, " ")
                .to_string(),
            Transformation::Wrap => soft_wrap(&string, 80).to_string(),
        }
    }
}

#[cfg(test)]
mod test_transformation {
    use super::Transformation;

    #[test]
    fn join() {
        let result = Transformation::Join.apply(
            "
who 
  lives
    in 
      a

pineapple?
"
            .trim()
            .to_string(),
        );
        assert_eq!(result, "who lives in a pineapple?")
    }

    #[test]
    fn wrap() {
        let result = Transformation::Wrap
            .apply("
who lives in a pineapple under the sea? Spongebob Squarepants! absorbent and yellow and porous is he? Spongebob Squarepants
"
            .trim().to_string());
        assert_eq!(result, "who lives in a pineapple under the sea? Spongebob Squarepants! absorbent and \nyellow and porous is he? Spongebob Squarepants")
    }
}
