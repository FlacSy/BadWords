//! Score text on every axis.
//!
//! Run: BADWORDS_ML_PATH=ml/models cargo run -p badwords-ml --example score -- "some text"

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = badwords_ml::ToxicityModel::open_located()?;
    println!("axes: {}", model.label_list());

    let args: Vec<String> = std::env::args().skip(1).collect();
    let texts: Vec<&str> = if args.is_empty() {
        vec![
            "you are a worthless idiot",
            "i will find you and hurt you",
            "have a lovely afternoon",
        ]
    } else {
        args.iter().map(String::as_str).collect()
    };

    for (text, scores) in texts.iter().zip(model.predict_batch(&texts)?) {
        println!("{text:?}");
        let mut ranked: Vec<(&str, f32)> = scores.iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (axis, value) in ranked {
            println!("   {axis:<18}{value:.4}");
        }
    }
    Ok(())
}
