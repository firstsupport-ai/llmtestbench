use std::collections::HashMap;

pub fn calculate_similarity<'a>(text1: &'a str, text2: &'a str) -> f64 {
    let frequencies = |text: &'a str| -> HashMap<&'a str, usize> {
        text.split_ascii_whitespace()
            .fold(HashMap::new(), |mut acc, word| {
                *acc.entry(word).or_insert(0) += 1;
                acc
            })
    };

    let freqs1 = frequencies(text1);
    let freqs2 = frequencies(text2);

    let magnitude = |freqs: &HashMap<&'a str, usize>| -> f64 {
        (freqs.values()
            .map(|&count| count * count)
            .sum::<usize>() as f64)
            .sqrt()
    };

    let mag1 = magnitude(&freqs1);
    let mag2 = magnitude(&freqs2);

    let dot_product: usize = freqs1.iter()
        .filter_map(|(&word, &count1)| {
            freqs2.get(word).map(|&count2| count1 * count2)
        })
        .sum();

    // Return cosine similarity
    if mag1 == 0.0 || mag2 == 0.0 {
        0.0
    } else {
        (dot_product as f64) / (mag1 * mag2)
    }
}
