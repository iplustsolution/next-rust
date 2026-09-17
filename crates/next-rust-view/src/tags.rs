//! Element macros: one macro per HTML element.
//!
//! Every macro accepts any mix of attributes ([`crate::attrs`]) and children
//! (anything implementing [`crate::View`]), separated by commas.

/// `<a>` element.
#[macro_export]
macro_rules! a {
    ($($part:expr),* $(,)?) => { $crate::Element::new("a")$(.with($part))* };
}

/// `<abbr>` element.
#[macro_export]
macro_rules! abbr {
    ($($part:expr),* $(,)?) => { $crate::Element::new("abbr")$(.with($part))* };
}

/// `<address>` element.
#[macro_export]
macro_rules! address {
    ($($part:expr),* $(,)?) => { $crate::Element::new("address")$(.with($part))* };
}

/// `<article>` element.
#[macro_export]
macro_rules! article {
    ($($part:expr),* $(,)?) => { $crate::Element::new("article")$(.with($part))* };
}

/// `<aside>` element.
#[macro_export]
macro_rules! aside {
    ($($part:expr),* $(,)?) => { $crate::Element::new("aside")$(.with($part))* };
}

/// `<audio>` element.
#[macro_export]
macro_rules! audio {
    ($($part:expr),* $(,)?) => { $crate::Element::new("audio")$(.with($part))* };
}

/// `<b>` element.
#[macro_export]
macro_rules! b {
    ($($part:expr),* $(,)?) => { $crate::Element::new("b")$(.with($part))* };
}

/// `<blockquote>` element.
#[macro_export]
macro_rules! blockquote {
    ($($part:expr),* $(,)?) => { $crate::Element::new("blockquote")$(.with($part))* };
}

/// `<body>` element.
#[macro_export]
macro_rules! body {
    ($($part:expr),* $(,)?) => { $crate::Element::new("body")$(.with($part))* };
}

/// `<button>` element.
#[macro_export]
macro_rules! button {
    ($($part:expr),* $(,)?) => { $crate::Element::new("button")$(.with($part))* };
}

/// `<canvas>` element.
#[macro_export]
macro_rules! canvas {
    ($($part:expr),* $(,)?) => { $crate::Element::new("canvas")$(.with($part))* };
}

/// `<caption>` element.
#[macro_export]
macro_rules! caption {
    ($($part:expr),* $(,)?) => { $crate::Element::new("caption")$(.with($part))* };
}

/// `<cite>` element.
#[macro_export]
macro_rules! cite {
    ($($part:expr),* $(,)?) => { $crate::Element::new("cite")$(.with($part))* };
}

/// `<code>` element.
#[macro_export]
macro_rules! code {
    ($($part:expr),* $(,)?) => { $crate::Element::new("code")$(.with($part))* };
}

/// `<colgroup>` element.
#[macro_export]
macro_rules! colgroup {
    ($($part:expr),* $(,)?) => { $crate::Element::new("colgroup")$(.with($part))* };
}

/// `<datalist>` element.
#[macro_export]
macro_rules! datalist {
    ($($part:expr),* $(,)?) => { $crate::Element::new("datalist")$(.with($part))* };
}

/// `<dd>` element.
#[macro_export]
macro_rules! dd {
    ($($part:expr),* $(,)?) => { $crate::Element::new("dd")$(.with($part))* };
}

/// `<del>` element.
#[macro_export]
macro_rules! del {
    ($($part:expr),* $(,)?) => { $crate::Element::new("del")$(.with($part))* };
}

/// `<details>` element.
#[macro_export]
macro_rules! details {
    ($($part:expr),* $(,)?) => { $crate::Element::new("details")$(.with($part))* };
}

/// `<dfn>` element.
#[macro_export]
macro_rules! dfn {
    ($($part:expr),* $(,)?) => { $crate::Element::new("dfn")$(.with($part))* };
}

/// `<dialog>` element.
#[macro_export]
macro_rules! dialog {
    ($($part:expr),* $(,)?) => { $crate::Element::new("dialog")$(.with($part))* };
}

/// `<div>` element.
#[macro_export]
macro_rules! div {
    ($($part:expr),* $(,)?) => { $crate::Element::new("div")$(.with($part))* };
}

/// `<dl>` element.
#[macro_export]
macro_rules! dl {
    ($($part:expr),* $(,)?) => { $crate::Element::new("dl")$(.with($part))* };
}

/// `<dt>` element.
#[macro_export]
macro_rules! dt {
    ($($part:expr),* $(,)?) => { $crate::Element::new("dt")$(.with($part))* };
}

/// `<em>` element.
#[macro_export]
macro_rules! em {
    ($($part:expr),* $(,)?) => { $crate::Element::new("em")$(.with($part))* };
}

/// `<fieldset>` element.
#[macro_export]
macro_rules! fieldset {
    ($($part:expr),* $(,)?) => { $crate::Element::new("fieldset")$(.with($part))* };
}

/// `<figcaption>` element.
#[macro_export]
macro_rules! figcaption {
    ($($part:expr),* $(,)?) => { $crate::Element::new("figcaption")$(.with($part))* };
}

/// `<figure>` element.
#[macro_export]
macro_rules! figure {
    ($($part:expr),* $(,)?) => { $crate::Element::new("figure")$(.with($part))* };
}

/// `<footer>` element.
#[macro_export]
macro_rules! footer {
    ($($part:expr),* $(,)?) => { $crate::Element::new("footer")$(.with($part))* };
}

/// `<form>` element.
#[macro_export]
macro_rules! form {
    ($($part:expr),* $(,)?) => { $crate::Element::new("form")$(.with($part))* };
}

/// `<h1>` element.
#[macro_export]
macro_rules! h1 {
    ($($part:expr),* $(,)?) => { $crate::Element::new("h1")$(.with($part))* };
}

/// `<h2>` element.
#[macro_export]
macro_rules! h2 {
    ($($part:expr),* $(,)?) => { $crate::Element::new("h2")$(.with($part))* };
}

/// `<h3>` element.
#[macro_export]
macro_rules! h3 {
    ($($part:expr),* $(,)?) => { $crate::Element::new("h3")$(.with($part))* };
}

/// `<h4>` element.
#[macro_export]
macro_rules! h4 {
    ($($part:expr),* $(,)?) => { $crate::Element::new("h4")$(.with($part))* };
}

/// `<h5>` element.
#[macro_export]
macro_rules! h5 {
    ($($part:expr),* $(,)?) => { $crate::Element::new("h5")$(.with($part))* };
}

/// `<h6>` element.
#[macro_export]
macro_rules! h6 {
    ($($part:expr),* $(,)?) => { $crate::Element::new("h6")$(.with($part))* };
}

/// `<head>` element.
#[macro_export]
macro_rules! head {
    ($($part:expr),* $(,)?) => { $crate::Element::new("head")$(.with($part))* };
}

/// `<header>` element.
#[macro_export]
macro_rules! header {
    ($($part:expr),* $(,)?) => { $crate::Element::new("header")$(.with($part))* };
}

/// `<hgroup>` element.
#[macro_export]
macro_rules! hgroup {
    ($($part:expr),* $(,)?) => { $crate::Element::new("hgroup")$(.with($part))* };
}

/// `<html>` element.
#[macro_export]
macro_rules! html {
    ($($part:expr),* $(,)?) => { $crate::Element::new("html")$(.with($part))* };
}

/// `<i>` element.
#[macro_export]
macro_rules! i {
    ($($part:expr),* $(,)?) => { $crate::Element::new("i")$(.with($part))* };
}

/// `<iframe>` element.
#[macro_export]
macro_rules! iframe {
    ($($part:expr),* $(,)?) => { $crate::Element::new("iframe")$(.with($part))* };
}

/// `<ins>` element.
#[macro_export]
macro_rules! ins {
    ($($part:expr),* $(,)?) => { $crate::Element::new("ins")$(.with($part))* };
}

/// `<kbd>` element.
#[macro_export]
macro_rules! kbd {
    ($($part:expr),* $(,)?) => { $crate::Element::new("kbd")$(.with($part))* };
}

/// `<label>` element.
#[macro_export]
macro_rules! label {
    ($($part:expr),* $(,)?) => { $crate::Element::new("label")$(.with($part))* };
}

/// `<legend>` element.
#[macro_export]
macro_rules! legend {
    ($($part:expr),* $(,)?) => { $crate::Element::new("legend")$(.with($part))* };
}

/// `<li>` element.
#[macro_export]
macro_rules! li {
    ($($part:expr),* $(,)?) => { $crate::Element::new("li")$(.with($part))* };
}

/// `<main>` element.
#[macro_export]
macro_rules! main {
    ($($part:expr),* $(,)?) => { $crate::Element::new("main")$(.with($part))* };
}

/// `<mark>` element.
#[macro_export]
macro_rules! mark {
    ($($part:expr),* $(,)?) => { $crate::Element::new("mark")$(.with($part))* };
}

/// `<menu>` element.
#[macro_export]
macro_rules! menu {
    ($($part:expr),* $(,)?) => { $crate::Element::new("menu")$(.with($part))* };
}

/// `<meter>` element.
#[macro_export]
macro_rules! meter {
    ($($part:expr),* $(,)?) => { $crate::Element::new("meter")$(.with($part))* };
}

/// `<nav>` element.
#[macro_export]
macro_rules! nav {
    ($($part:expr),* $(,)?) => { $crate::Element::new("nav")$(.with($part))* };
}

/// `<noscript>` element.
#[macro_export]
macro_rules! noscript {
    ($($part:expr),* $(,)?) => { $crate::Element::new("noscript")$(.with($part))* };
}

/// `<object>` element.
#[macro_export]
macro_rules! object {
    ($($part:expr),* $(,)?) => { $crate::Element::new("object")$(.with($part))* };
}

/// `<ol>` element.
#[macro_export]
macro_rules! ol {
    ($($part:expr),* $(,)?) => { $crate::Element::new("ol")$(.with($part))* };
}

/// `<optgroup>` element.
#[macro_export]
macro_rules! optgroup {
    ($($part:expr),* $(,)?) => { $crate::Element::new("optgroup")$(.with($part))* };
}

/// `<option>` element.
#[macro_export]
macro_rules! option {
    ($($part:expr),* $(,)?) => { $crate::Element::new("option")$(.with($part))* };
}

/// `<output>` element.
#[macro_export]
macro_rules! output {
    ($($part:expr),* $(,)?) => { $crate::Element::new("output")$(.with($part))* };
}

/// `<p>` element.
#[macro_export]
macro_rules! p {
    ($($part:expr),* $(,)?) => { $crate::Element::new("p")$(.with($part))* };
}

/// `<picture>` element.
#[macro_export]
macro_rules! picture {
    ($($part:expr),* $(,)?) => { $crate::Element::new("picture")$(.with($part))* };
}

/// `<pre>` element.
#[macro_export]
macro_rules! pre {
    ($($part:expr),* $(,)?) => { $crate::Element::new("pre")$(.with($part))* };
}

/// `<progress>` element.
#[macro_export]
macro_rules! progress {
    ($($part:expr),* $(,)?) => { $crate::Element::new("progress")$(.with($part))* };
}

/// `<q>` element.
#[macro_export]
macro_rules! q {
    ($($part:expr),* $(,)?) => { $crate::Element::new("q")$(.with($part))* };
}

/// `<rp>` element.
#[macro_export]
macro_rules! rp {
    ($($part:expr),* $(,)?) => { $crate::Element::new("rp")$(.with($part))* };
}

/// `<rt>` element.
#[macro_export]
macro_rules! rt {
    ($($part:expr),* $(,)?) => { $crate::Element::new("rt")$(.with($part))* };
}

/// `<ruby>` element.
#[macro_export]
macro_rules! ruby {
    ($($part:expr),* $(,)?) => { $crate::Element::new("ruby")$(.with($part))* };
}

/// `<s>` element.
#[macro_export]
macro_rules! s {
    ($($part:expr),* $(,)?) => { $crate::Element::new("s")$(.with($part))* };
}

/// `<samp>` element.
#[macro_export]
macro_rules! samp {
    ($($part:expr),* $(,)?) => { $crate::Element::new("samp")$(.with($part))* };
}

/// `<script>` element.
#[macro_export]
macro_rules! script {
    ($($part:expr),* $(,)?) => { $crate::Element::new("script")$(.with($part))* };
}

/// `<section>` element.
#[macro_export]
macro_rules! section {
    ($($part:expr),* $(,)?) => { $crate::Element::new("section")$(.with($part))* };
}

/// `<select>` element.
#[macro_export]
macro_rules! select {
    ($($part:expr),* $(,)?) => { $crate::Element::new("select")$(.with($part))* };
}

/// `<slot>` element.
#[macro_export]
macro_rules! slot {
    ($($part:expr),* $(,)?) => { $crate::Element::new("slot")$(.with($part))* };
}

/// `<small>` element.
#[macro_export]
macro_rules! small {
    ($($part:expr),* $(,)?) => { $crate::Element::new("small")$(.with($part))* };
}

/// `<span>` element.
#[macro_export]
macro_rules! span {
    ($($part:expr),* $(,)?) => { $crate::Element::new("span")$(.with($part))* };
}

/// `<strong>` element.
#[macro_export]
macro_rules! strong {
    ($($part:expr),* $(,)?) => { $crate::Element::new("strong")$(.with($part))* };
}

/// `<style>` element.
#[macro_export]
macro_rules! style {
    ($($part:expr),* $(,)?) => { $crate::Element::new("style")$(.with($part))* };
}

/// `<sub>` element.
#[macro_export]
macro_rules! sub {
    ($($part:expr),* $(,)?) => { $crate::Element::new("sub")$(.with($part))* };
}

/// `<summary>` element.
#[macro_export]
macro_rules! summary {
    ($($part:expr),* $(,)?) => { $crate::Element::new("summary")$(.with($part))* };
}

/// `<sup>` element.
#[macro_export]
macro_rules! sup {
    ($($part:expr),* $(,)?) => { $crate::Element::new("sup")$(.with($part))* };
}

/// `<table>` element.
#[macro_export]
macro_rules! table {
    ($($part:expr),* $(,)?) => { $crate::Element::new("table")$(.with($part))* };
}

/// `<tbody>` element.
#[macro_export]
macro_rules! tbody {
    ($($part:expr),* $(,)?) => { $crate::Element::new("tbody")$(.with($part))* };
}

/// `<td>` element.
#[macro_export]
macro_rules! td {
    ($($part:expr),* $(,)?) => { $crate::Element::new("td")$(.with($part))* };
}

/// `<template>` element.
#[macro_export]
macro_rules! template {
    ($($part:expr),* $(,)?) => { $crate::Element::new("template")$(.with($part))* };
}

/// `<textarea>` element.
#[macro_export]
macro_rules! textarea {
    ($($part:expr),* $(,)?) => { $crate::Element::new("textarea")$(.with($part))* };
}

/// `<tfoot>` element.
#[macro_export]
macro_rules! tfoot {
    ($($part:expr),* $(,)?) => { $crate::Element::new("tfoot")$(.with($part))* };
}

/// `<th>` element.
#[macro_export]
macro_rules! th {
    ($($part:expr),* $(,)?) => { $crate::Element::new("th")$(.with($part))* };
}

/// `<thead>` element.
#[macro_export]
macro_rules! thead {
    ($($part:expr),* $(,)?) => { $crate::Element::new("thead")$(.with($part))* };
}

/// `<time>` element.
#[macro_export]
macro_rules! time {
    ($($part:expr),* $(,)?) => { $crate::Element::new("time")$(.with($part))* };
}

/// `<title>` element.
#[macro_export]
macro_rules! title {
    ($($part:expr),* $(,)?) => { $crate::Element::new("title")$(.with($part))* };
}

/// `<tr>` element.
#[macro_export]
macro_rules! tr {
    ($($part:expr),* $(,)?) => { $crate::Element::new("tr")$(.with($part))* };
}

/// `<u>` element.
#[macro_export]
macro_rules! u {
    ($($part:expr),* $(,)?) => { $crate::Element::new("u")$(.with($part))* };
}

/// `<ul>` element.
#[macro_export]
macro_rules! ul {
    ($($part:expr),* $(,)?) => { $crate::Element::new("ul")$(.with($part))* };
}

/// `<var>` element.
#[macro_export]
macro_rules! var {
    ($($part:expr),* $(,)?) => { $crate::Element::new("var")$(.with($part))* };
}

/// `<video>` element.
#[macro_export]
macro_rules! video {
    ($($part:expr),* $(,)?) => { $crate::Element::new("video")$(.with($part))* };
}

/// `<svg>` element.
#[macro_export]
macro_rules! svg {
    ($($part:expr),* $(,)?) => { $crate::Element::new("svg")$(.with($part))* };
}

/// `<g>` element.
#[macro_export]
macro_rules! g {
    ($($part:expr),* $(,)?) => { $crate::Element::new("g")$(.with($part))* };
}

/// `<path>` element.
#[macro_export]
macro_rules! path {
    ($($part:expr),* $(,)?) => { $crate::Element::new("path")$(.with($part))* };
}

/// `<circle>` element.
#[macro_export]
macro_rules! circle {
    ($($part:expr),* $(,)?) => { $crate::Element::new("circle")$(.with($part))* };
}

/// `<rect>` element.
#[macro_export]
macro_rules! rect {
    ($($part:expr),* $(,)?) => { $crate::Element::new("rect")$(.with($part))* };
}

/// `<ellipse>` element.
#[macro_export]
macro_rules! ellipse {
    ($($part:expr),* $(,)?) => { $crate::Element::new("ellipse")$(.with($part))* };
}

/// `<polyline>` element.
#[macro_export]
macro_rules! polyline {
    ($($part:expr),* $(,)?) => { $crate::Element::new("polyline")$(.with($part))* };
}

/// `<polygon>` element.
#[macro_export]
macro_rules! polygon {
    ($($part:expr),* $(,)?) => { $crate::Element::new("polygon")$(.with($part))* };
}

/// `<defs>` element.
#[macro_export]
macro_rules! defs {
    ($($part:expr),* $(,)?) => { $crate::Element::new("defs")$(.with($part))* };
}

/// `<symbol>` element.
#[macro_export]
macro_rules! symbol {
    ($($part:expr),* $(,)?) => { $crate::Element::new("symbol")$(.with($part))* };
}

/// `<use>` element.
#[macro_export]
macro_rules! use_ {
    ($($part:expr),* $(,)?) => { $crate::Element::new("use")$(.with($part))* };
}

/// `<clipPath>` element.
#[macro_export]
macro_rules! clipPath {
    ($($part:expr),* $(,)?) => { $crate::Element::new("clipPath")$(.with($part))* };
}

/// `<linearGradient>` element.
#[macro_export]
macro_rules! linearGradient {
    ($($part:expr),* $(,)?) => { $crate::Element::new("linearGradient")$(.with($part))* };
}

/// `<radialGradient>` element.
#[macro_export]
macro_rules! radialGradient {
    ($($part:expr),* $(,)?) => { $crate::Element::new("radialGradient")$(.with($part))* };
}

/// `<stop>` element.
#[macro_export]
macro_rules! stop {
    ($($part:expr),* $(,)?) => { $crate::Element::new("stop")$(.with($part))* };
}

/// `<area>` void element (children are ignored).
#[macro_export]
macro_rules! area {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("area")$(.with($part))* };
}

/// `<base>` void element (children are ignored).
#[macro_export]
macro_rules! base {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("base")$(.with($part))* };
}

/// `<br>` void element (children are ignored).
#[macro_export]
macro_rules! br {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("br")$(.with($part))* };
}

/// `<col>` void element (children are ignored).
#[macro_export]
macro_rules! col {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("col")$(.with($part))* };
}

/// `<embed>` void element (children are ignored).
#[macro_export]
macro_rules! embed {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("embed")$(.with($part))* };
}

/// `<hr>` void element (children are ignored).
#[macro_export]
macro_rules! hr {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("hr")$(.with($part))* };
}

/// `<img>` void element (children are ignored).
#[macro_export]
macro_rules! img {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("img")$(.with($part))* };
}

/// `<input>` void element (children are ignored).
#[macro_export]
macro_rules! input {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("input")$(.with($part))* };
}

/// `<link>` void element (children are ignored).
#[macro_export]
macro_rules! link {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("link")$(.with($part))* };
}

/// `<meta>` void element (children are ignored).
#[macro_export]
macro_rules! meta {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("meta")$(.with($part))* };
}

/// `<source>` void element (children are ignored).
#[macro_export]
macro_rules! source {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("source")$(.with($part))* };
}

/// `<track>` void element (children are ignored).
#[macro_export]
macro_rules! track {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("track")$(.with($part))* };
}

/// `<wbr>` void element (children are ignored).
#[macro_export]
macro_rules! wbr {
    ($($part:expr),* $(,)?) => { $crate::Element::new_void("wbr")$(.with($part))* };
}

/// Names of all generated element macros.
pub const TAGS: &[&str] = &[
    "a",
    "abbr",
    "address",
    "article",
    "aside",
    "audio",
    "b",
    "blockquote",
    "body",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "colgroup",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "html",
    "i",
    "iframe",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "main",
    "mark",
    "menu",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "section",
    "select",
    "slot",
    "small",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "u",
    "ul",
    "var",
    "video",
    "svg",
    "g",
    "path",
    "circle",
    "rect",
    "ellipse",
    "polyline",
    "polygon",
    "defs",
    "symbol",
    "use",
    "clipPath",
    "linearGradient",
    "radialGradient",
    "stop",
    "area",
    "base",
    "br",
    "col",
    "embed",
    "hr",
    "img",
    "input",
    "link",
    "meta",
    "source",
    "track",
    "wbr",
];

/// Names of void elements.
pub const VOID_TAGS: &[&str] =
    &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];
