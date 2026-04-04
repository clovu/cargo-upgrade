mod error;

use error::Result;

fn main() -> Result<()> {
    // load env profile
    dotenvy::dotenv()?;

    Ok(())
}
