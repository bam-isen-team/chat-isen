import type { TargetedSubmitEvent } from 'preact';
import Logo from '../../assets/chat-isen-logo.png';
import './Login.css';
import { useState } from 'preact/hooks';
import { use } from 'react-dom/src';

const API_BASE_URL = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '');

interface LoginResponse {
	acess_token: string;
	user: {
		id: string;
		username: string;
		email: string;
		display_name: string;
	};
}

interface LoginError {
	error: string;
}

export function Login() {

	const [errorMessage, setErrorMessage] = useState('');
	const [successMessage, setSucessMessage] = useState('');
	const [isSubmitting, setIsSubmitting] = useState(false);

	async function handleSubmit(event: TargetedSubmitEvent<HTMLFormElement>) {
		event.preventDefault();
		const loginData = new LoginData(event.currentTarget);

		setErrorMessage('');
		setSucessMessage('');
		setIsSubmitting(true);

		try {
			const response = await fetch(`${API_BASE_URL}/auth/login`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					email: loginData.get('email')?.toString() ?? '',
					password: loginData.get('password')?.toString() ?? '',
				}),
			});

			if (response.status === 201) {
				const result: LoginResponse = await response.json();
				setSucessMessage(`Registration successful.Welcome, ${result.user.display_name}!`);
				return;
			}
			if (response.status === 400) {
				const result: LoginError = await response.json();
				setErrorMessage(result.error);
				return;
			}
			if (response.status === 409) {
				setErrorMessage('Username or password invalid.');
				return;
			}

			setErrorMessage(`Login failed (HTTP $ {response.status}).`);
		}
		catch (error) {
			setErrorMessage(
				error instanceof Error ? `Unable to login: $ {error.message}` : `Unable to login.`,
			);
		}
		finally {
			setIsSubmitting(false);
		}
	}

	return (
		<div class="login">
			<img src={Logo} alt="ChatISEN logo" height="160" width="160" />
			<h1>Welcome to Your ISEN Chat App</h1>
			<section onSubmit={handleSubmit}>
				<h2>Login</h2>
				<div class="loginform">
					<label for="username">Username:</label>
					<input type="text" id="username" name="username" required />
					<label for="password">Password:</label>
					<input type="password" id="password" name="password" required />
					<button type="submit" disabled={isSubmitting}>{isSubmitting ? 'Logging...' : 'Log In'}
					</button>
					{errorMessage && <p role="alert">{errorMessage}</p>}
					{successMessage && <p role="status">{successMessage}</p>}
					<h3>Not Registered? <a href="/registration">Sign Up</a></h3>
				</div>
				
			</section>
			
		</div>
	);
}