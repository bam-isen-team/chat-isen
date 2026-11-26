import Logo from '../../assets/chat-isen-logo.png';
import './Login.css';

export function Login() {
	return (
		<div class="login">
			<img src={Logo} alt="Preact logo" height="160" width="160" />
			<h1>Welcome to Your ISEN Chat App</h1>
			<section>
				<h2>Login</h2>
				<div class="loginform">
					<label for="username">Username:</label>
					<input type="text" id="username" name="username" required />
					<label for="password">Password:</label>
					<input type="password" id="password" name="password" required />
					<button type="submit">Log In</button>
					<h3>Not Registered? <a href="/registration">Sign Up</a></h3>
				</div>
				
			</section>
			
		</div>
	);
}