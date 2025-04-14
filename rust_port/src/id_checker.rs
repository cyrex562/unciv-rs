use std::char;

/// This struct checks whether a Game- or Player-ID matches the old or new format.
/// If old format is used, checks are skipped and input is returned.
/// If new format is detected, prefix and checkDigit are checked and UUID returned.
///
/// All input is returned trimmed.
///
/// New format:
/// G-UUID-CheckDigit for Game IDs
/// P-UUID-CheckDigit for Player IDs
///
/// Example:
/// 2ddb3a34-0699-4126-b7a5-38603e665928
/// Same ID in proposed new Player-ID format:
/// P-2ddb3a34-0699-4126-b7a5-38603e665928-5
/// Same ID in proposed new Game-ID format:
/// G-2ddb3a34-0699-4126-b7a5-38603e665928-5
pub struct IdChecker;

impl IdChecker {
    /// Check and return a player UUID
    pub fn check_and_return_player_uuid(player_id: &str) -> Result<String, String> {
        Self::check_and_return_uuid(player_id, "P")
    }

    /// Check and return a game UUID
    pub fn check_and_return_game_uuid(game_id: &str) -> Result<String, String> {
        Self::check_and_return_uuid(game_id, "G")
    }

    /// Internal function to check and return a UUID with a specific prefix
    fn check_and_return_uuid(id: &str, prefix: &str) -> Result<String, String> {
        let trimmed_id = id.trim();

        if trimmed_id.len() == 40 { // length of a UUID (36) with pre- and postfix
            if !trimmed_id.to_uppercase().starts_with(prefix) {
                return Err(format!("Not a valid ID. Does not start with prefix {}", prefix));
            }

            let check_digit = &trimmed_id[trimmed_id.len() - 1..];
            // remember, the format is: P-9e37e983-a676-4ecc-800e-ef8ec721a9b9-5
            let shortened_id = &trimmed_id[2..38];
            let calculated_check_digit = Self::get_check_digit(shortened_id).to_string();

            if calculated_check_digit != check_digit {
                return Err("Not a valid ID. Checkdigit invalid.".to_string());
            }

            Ok(shortened_id.to_string())
        } else if trimmed_id.len() == 36 {
            Ok(trimmed_id.to_string())
        } else {
            Err("Not a valid ID. Wrong length.".to_string())
        }
    }

    /// Calculate check digit for a UUID
    /// Adapted from https://wiki.openmrs.org/display/docs/Check+Digit+Algorithm
    pub fn get_check_digit(uuid: &str) -> i32 {
        // allowable characters within identifier
        const VALID_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVYWXZ-";

        // remove leading or trailing whitespace, convert to uppercase
        let id_without_checkdigit = uuid.trim().to_uppercase();

        // this will be a running total
        let mut sum = 0;

        // loop through digits from right to left
        for (i, ch) in id_without_checkdigit.chars().rev().enumerate() {
            // throw exception for invalid characters
            if !VALID_CHARS.contains(ch) {
                panic!("{} is an invalid character", ch);
            }

            // our "digit" is calculated using ASCII value - 48
            let digit = ch as i32 - 48;

            // weight will be the current digit's contribution to the running total
            let weight = if i % 2 == 0 {
                // for alternating digits starting with the rightmost, we
                // use our formula this is the same as multiplying x 2 and
                // adding digits together for values 0 to 9. Using the
                // following formula allows us to gracefully calculate a
                // weight for non-numeric "digits" as well (from their
                // ASCII value - 48).
                (2 * digit) - (digit / 5) * 9
            } else {
                // even-positioned digits just contribute their ascii value minus 48
                digit
            };

            // keep a running total of weights
            sum += weight;
        }

        // avoid sum less than 10 (if characters below "0" allowed, this could happen)
        sum = sum.abs() + 10;

        // check digit is amount needed to reach next number divisible by ten
        (10 - (sum % 10)) % 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_player_id() {
        let result = IdChecker::check_and_return_player_uuid("P-2ddb3a34-0699-4126-b7a5-38603e665928-5");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2ddb3a34-0699-4126-b7a5-38603e665928");
    }

    #[test]
    fn test_valid_game_id() {
        let result = IdChecker::check_and_return_game_uuid("G-2ddb3a34-0699-4126-b7a5-38603e665928-5");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2ddb3a34-0699-4126-b7a5-38603e665928");
    }

    #[test]
    fn test_old_format_uuid() {
        let uuid = "2ddb3a34-0699-4126-b7a5-38603e665928";
        let result = IdChecker::check_and_return_player_uuid(uuid);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), uuid);
    }

    #[test]
    fn test_invalid_prefix() {
        let result = IdChecker::check_and_return_player_uuid("X-2ddb3a34-0699-4126-b7a5-38603e665928-5");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Does not start with prefix"));
    }

    #[test]
    fn test_invalid_length() {
        let result = IdChecker::check_and_return_player_uuid("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Wrong length"));
    }
}