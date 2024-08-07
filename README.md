# Blog

> I have another repo for this blog except the backend was written in Bun + Elysia.js. The reason for this rewrite is that Bun took up too much memory on Railway so I decided to rewrite it in Rust

-   Simple blog site written in Axum (Rust), HTMX, and Handlebars templating
-   Posts are written and stored locally, then the server parsed and stores each one in Postgres
-   Styled using TailwindCSS

### TODO:

-   [ ] implement infinite scroll when I have more articles
