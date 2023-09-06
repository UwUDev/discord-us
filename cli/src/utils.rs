use rand::{distributions::Alphanumeric, Rng};
use discord_us::signal::{ProgressionRange};

pub fn create_random_password(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn to_progress_bar(
    ranges: Vec<ProgressionRange<u64>>,
    total: u64,
    size: usize,
    character_loaded: char,
    character_unloaded: char
) -> String {
    let mut result = String::with_capacity(size);

    let part_range = total / size as u64;
    let mut range_cursor = 0usize;
    let mut c = 0u64;
    let step = part_range / 2;

    while c < total {
        if let Some(range) = ranges.get(range_cursor) {
            if c + step >= range.range_start && c + step < range.range_end {
                result.push(character_loaded);
            } else {
                result.push(character_unloaded);
                if c >= range.range_end {
                    range_cursor += 1;
                }
            }
        } else {
            result.push(character_unloaded);
        }

        c += part_range;
    };

    result
}

// Do test
#[cfg(test)]
mod tests {
    use crate::utils::to_progress_bar;
    use discord_us::signal::{ProgressionRange};

    #[test]
    fn test_progress_bar() {
        let ranges = vec![
            ProgressionRange::of(241, 401),

            ProgressionRange::of(608, 754),
        ];

        let result = to_progress_bar(ranges, 1000, 100, '#', '-');

        println!("{}", result);
    }
}
