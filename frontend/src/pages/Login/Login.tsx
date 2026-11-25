import Logo from '../../assets/chat-isen-logo.png';
import './Login.css';

export function Login() {
	return (
		<div class="login">
			<img src={Logo} alt="Preact logo" height="160" width="160" />
			<h1>Welcome to Your ISEN Chat App</h1>
			<section>
				<h2>Login</h2>
				<h3>Username</h3>
				<input type="text" placeholder="Enter your username" class="input" />
				<h3>Password</h3>
				<input type="password" placeholder="Enter your password" class="input" />
				<h3></h3>
				<button class="button"><a href="/chat">Log In</a></button>
				<h4>Not Registered? <a href="/registration">Sign Up</a></h4>
			</section>
			
		</div>
	);
}