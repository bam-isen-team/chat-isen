import Logo from '../../assets/chat_icon.png';
import './style.css';

export function Login() {
	return (
		<div class="login">
			<img src={Logo} alt="Preact logo" height="160" width="160" />
			<h1>Welcome to Your Chat App</h1>
			<section>
				<Resource
					title="Log In"
				/>
			</section>
			<h4>Not Registered? <a href="../Register/register">Sign Up</a></h4>
		</div>
	);
}

function Resource(props) {
	return (
		<a href={props.href} target="_blank" class="resource">
			<h2>{props.title}</h2>
			<h3 style={{ textAlign: 'left', fontWeight: 'normal', fontSize: '1rem' }}>Username</h3>
			<input type="text" placeholder="Enter your username" class="input" />
			<h3 style={{ textAlign: 'left', fontWeight: 'normal', fontSize: '1rem' }}>Password</h3>
			<input type="password" placeholder="Enter your password" class="input" />
			<button class="button"><a href="https://google.com">Log In</a></button>
		</a>
	);
}
