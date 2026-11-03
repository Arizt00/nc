# Search discovery for nC and ncc

The public website at https://ncc-lenguage.org identifies **nC** as the programming language and **ncc** as its compiler. Public text, navigation and metadata are in English. This website still documents bootstrap edition V81; a website change does not promote a compiler milestone.

## Entry points

| Page | Reader's task |
| --- | --- |
| `/` | Identify the nC programming language and ncc compiler |
| `/nc-programming-language` | Understand language design, memory and the machine layer |
| `/ncc-compiler` | Download ncc, compile an example and understand bootstrap limits |
| `/library` | Find language guides and compiler architecture references |
| `/downloads` | Select the Windows or Linux compiler kit |

The 16 documentation chapters retain their URLs. Legacy Spanish routes still redirect to the English edition. The sitemap contains 21 canonical pages; redirect pages and the noindex 404 page are excluded. Internal page links, search results, social URLs and canonical URLs use the hosting service's extensionless routes.

## Source and checks

`tools/publishing/seo.mjs` holds the page-specific titles, descriptions, language/compiler overviews and structured metadata. `build-site.mjs` combines these with the existing manuals. The website identifies its name with `WebSite`, the language with `ComputerLanguage`, and the compiler repository with `SoftwareSourceCode`. Each page has a `WebPage` record. Interior pages have visible breadcrumbs and matching `BreadcrumbList` data.

Run from the public export root:

```sh
node tools/publishing/build-site.mjs
node tools/publishing/check-public.mjs
node tools/publishing/check-seo.mjs
```

The checks cover local references, English language declarations, heading structure, search destinations, sitemap coverage, canonical URLs, unique titles and descriptions, parseable JSON-LD, breadcrumb identity and the 404 indexing policy. The metadata describes visible content; it does not invent ratings, reviews or support claims. Its schema validity is not a claim of eligibility for a particular Google rich result.

## Search Console and limits

The domain property was verified and the sitemap submitted on 3 November 2026. A homepage indexing request was accepted on that date. Google periodically processes the same sitemap URL, including newly added pages. Search Console remains the place to inspect crawl and indexing status; discovery, indexing and ranking are separate states.

Short queries such as `nC` and `ncc` have meanings outside this project. Titles, headings and links therefore pair the names with their specific subjects, such as `nC programming language` and `ncc compiler`. These changes make the project easier to identify; they do not guarantee a ranking or an indexing date.

References: [Google title links](https://developers.google.com/search/docs/appearance/title-link), [site names](https://developers.google.com/search/docs/appearance/site-names), [breadcrumbs](https://developers.google.com/search/docs/appearance/structured-data/breadcrumb), and [crawl requests](https://developers.google.com/search/docs/crawling-indexing/ask-google-to-recrawl).
