/*
    Copyright 2025 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Utility functions and helpers.

/// Extract the ordinal (node id) from a string with format `prefix-{ordinal}`.
pub fn extract_ordinal_from_string(value: &str) -> Option<u32> {
    value
        .match_indices('-')
        .next_back()
        .map(|(last_dash_index, _)| value.split_at(last_dash_index + 1).1)
        .and_then(|ordinal_string| {
            ordinal_string
                .parse::<u32>()
                .inspect_err(|e| log::debug!("Failed to parse ordinal '{ordinal_string}': {e}"))
                .ok()
        })
}

mod test {
    //! Configuration tests.

    #[test]
    fn test_happy_ordinal_extraction() {
        let ordinal = super::extract_ordinal_from_string("clacheless-1");
        assert_eq!(ordinal, Some(1));
        let ordinal = super::extract_ordinal_from_string("clacheless-123");
        assert_eq!(ordinal, Some(123));
    }
}
