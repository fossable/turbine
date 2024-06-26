use anyhow::Result;
use cached::proc_macro::once;

/// Generate a simple SVG badge with the given attributes.
pub fn _generate(title: &str, value: &str) -> String {
    format!(
        r###"
        <svg
        	xmlns="http://www.w3.org/2000/svg"
        	xmlns:xlink="http://www.w3.org/1999/xlink" width="116" height="20" role="img" aria-label="{0}: {1}">
        	<title>{0}: {1}</title>
        	<linearGradient id="s" x2="0" y2="100%">
        		<stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
        		<stop offset="1" stop-opacity=".1"/>
        	</linearGradient>
        	<clipPath id="r">
        		<rect width="116" height="20" rx="3" fill="#fff"/>
        	</clipPath>
        	<g clip-path="url(#r)">
        		<rect width="53" height="20" fill="#555"/>
        		<rect x="53" width="63" height="20" fill="#97ca00"/>
        		<rect width="116" height="20" fill="url(#s)"/>
        	</g>
        	<g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="110">
        		<text aria-hidden="true" x="275" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="430">{0}</text>
        		<text x="275" y="140" transform="scale(.1)" fill="#fff" textLength="430">{0}</text>
        		<text aria-hidden="true" x="835" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="530">{1}</text>
        		<text x="835" y="140" transform="scale(.1)" fill="#fff" textLength="530">{1}</text>
        	</g>
        </svg>
    "###,
        title,
        value,
        // title.len() * 10,
        // value.len() * 10,
    )
}

/// Load a badge from shields.io. TODO: replace this with a custom generator.
#[once(result = true)]
pub async fn generate(title: &str, value: &str) -> Result<String> {
    Ok(reqwest::get(format!(
        "https://img.shields.io/badge/{}-{}-green",
        title, value
    ))
    .await?
    .text()
    .await?)
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_generate() {
        super::generate("test", "value").await.unwrap();
    }
}
