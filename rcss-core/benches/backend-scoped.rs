use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quote::spanned::Spanned;
#[cfg(not(all(feature = "stylers", feature = "lightningcss")))]
compile_error!("You should enable stylers or lightningcss to use benchmarks");

fn backends_benchmark(c: &mut Criterion) {
    // style from stylers github README
    let css_from = r##"button {
            background-color: green;
            border-radius: 8px;
            border-style: none;
            box-sizing: border-box;
            color: yellow;
            cursor: pointer;
            display: inline-block;
            font-family: "Haas Grot Text R Web", "Helvetica Neue", Helvetica, Arial, sans-serif;
            font-size: 14px;
            font-weight: 500;
            height: 40px;
            line-height: 20px;
            list-style: none;
            margin: 0;
            outline: none;
            padding: 10px 16px;
            position: relative;
            text-align: center;
            text-decoration: none;
            transition: color 100ms;
            vertical-align: baseline;
            user-select: none;
            -webkit-user-select: none;
        }
        button:hover{
            background-color: yellow;
            color: green;
        }
        #two{
            color: blue;
        }
        div.one{
            color: red;
            content: raw_str("hello");
            font: "1.3em/1.2" Arial, Helvetica, sans-serif;
        }
        div {
            border: 1px solid black;
            margin: 25px 50px 75px 100px;
            background-color: lightblue;
        }
        h2 {
            color: purple;
        }
        @media only screen and (max-width: 1000px) {
            h3 {
                background-color: lightblue;
                color: blue
            }
        }
    "##;
    // Stylers backend
    c.bench_function("stylers", |iter| {
        iter.iter(|| {
            let _ = rcss_core::CssProcessor::new(
                rcss_core::CssPreprocessor::StylersCore,
                rcss_core::CssEmbeding::Scoped,
            )
            .process_style(black_box(css_from));
        })
    });
    // lightningcss backend
    c.bench_function("lightningcss", |iter| {
        iter.iter(|| {
            let _ = rcss_core::CssProcessor::new(
                rcss_core::CssPreprocessor::LightningCss,
                rcss_core::CssEmbeding::Scoped,
            )
            .process_style(black_box(css_from));
        })
    });

    // lightningcss with source_text recovered from preparsed tokenstream
    c.bench_function("lightningcss_source_text", |iter| {
        iter.iter(|| {
            let tt: proc_macro2::TokenStream = black_box(css_from.parse().unwrap());
            let css_from = tt.__span().source_text().unwrap();
            let _ = rcss_core::CssProcessor::new(
                rcss_core::CssPreprocessor::StylersCore,
                rcss_core::CssEmbeding::Scoped,
            )
            .process_style(black_box(&css_from));
        })
    });
    // stylers with source_text recovered from preparsed tokenstream
    c.bench_function("stylers_source_text", |iter| {
        iter.iter(|| {
            let tt: proc_macro2::TokenStream = black_box(css_from.parse().unwrap());
            let class = stylers_core::Class::new("test".to_owned());
            let css_from = tt.__span().source_text().unwrap();
            _ = stylers_core::from_str(&css_from, &class);
        })
    });
    // stylers (within backend) and source text recovered from preparsed tokenstream
    c.bench_function("stylers_source_text_backend", |iter| {
        iter.iter(|| {
            let tt: proc_macro2::TokenStream = black_box(css_from.parse().unwrap());
            let css_from = tt.__span().source_text().unwrap();
            let _ = rcss_core::CssProcessor::new(
                rcss_core::CssPreprocessor::StylersCore,
                rcss_core::CssEmbeding::Scoped,
            )
            .process_style(black_box(&css_from));
        })
    });
    // stylers tokenstream version that tries to recover spaces
    c.bench_function("stylers_tokenstream", |iter| {
        iter.iter(|| {
            let tt: proc_macro2::TokenStream = black_box(css_from.parse().unwrap());
            let class = stylers_core::Class::new("test".to_owned());
            _ = stylers_core::from_ts(tt.into_iter(), &class, false);
        })
    });
}
criterion_group!(backends, backends_benchmark);
criterion_main!(backends);
