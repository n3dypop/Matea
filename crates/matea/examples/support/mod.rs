use matea::prelude::*;
pub fn launch<A: Model>(model: A) -> Result<(), matea::terminal::Error> {
    if std::env::args().any(|s| s == "--snapshot") {
        let runtime = Runtime::new(model, Size::new(72, 24), Theme::plain())?;
        println!("{}", runtime.frame()?.plain().trim_end());
        Ok(())
    } else {
        matea::run(model)
    }
}
