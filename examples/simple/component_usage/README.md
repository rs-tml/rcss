Example of simple component library usage.
It redefines button components with custom styles.
And manually collect styles and concatenate them.

To simplify this style collection, ine can use `rcss-layers` crate that can save extended styles into CSS `@layer`, or use `rcss-bundler` crate to bundle all CSS into a static file.

A layered approach is a bit different with complex selectors.

For example, if you have two selectors with pseudo-class in base class.
    
```css
.container {
    background-color: red;
}
.container:hover {
    background-color: blue;
}
```
And extend css:
```css
@rcss(extend ...);
.container {
    background-color: white;
}
```
The layered approach will use `background-color` for both `container` and `container:hover` selectors, while simple concatenation that we use in this example project will use `background-color:white` for `container` and keep `.container:hover` as is. 
