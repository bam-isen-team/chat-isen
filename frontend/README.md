# `create-preact`

<h2 align="center">
  <img height="256" width="256" src="./src/assets/preact.svg">
</h2>

<h3 align="center">Get started using Preact and Vite!</h3>

## Getting Started

-   `npm run dev` - Starts a dev server at http://localhost:5173/

    During development, Vite proxies `/auth` requests to the backend at
    `http://localhost:3000`. Start the backend before using registration.

-   `npm run build` - Builds for production, emitting to `dist/`. Prerenders all found routes in app to static HTML

    `VITE_API_BASE_URL` sets the API base URL at build time. Leave it unset to
    use a same-origin `/auth` path (for example, when a production reverse
    proxy forwards `/auth` to the backend). For a separately hosted API, set
    `VITE_API_BASE_URL` to its public URL before building. This value is public
    and must not contain secrets.

-   `npm run preview` - Starts a server at http://localhost:4173/ to test production build locally
