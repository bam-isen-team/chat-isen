import { LocationProvider, Router, Route, hydrate, prerender as ssr } from 'preact-iso';

import { Login } from './pages/Login/Login.jsx';
import { NotFound } from './pages/_404.jsx';
import { Registration } from './pages/Registration/Registration.jsx';

export function App() {
	return (
		<LocationProvider>
			<main>
				<Router>
					<Route path="/" component={Login} />
					<Route path="/registration" component={Registration} />
					<Route default component={NotFound} />
				</Router>
			</main>
		</LocationProvider>
	);
}

if (typeof window !== 'undefined') {
	hydrate(<App />, document.getElementById('app'));
}

export async function prerender(data) {
	return await ssr(<App {...data} />);
}
