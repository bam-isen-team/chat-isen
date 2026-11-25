import type { TargetedSubmitEvent } from 'preact';
import './Registration.css';

export function Registration() {
    async function handleSubmit(event: TargetedSubmitEvent<HTMLFormElement>) {
        event.preventDefault();
        const formData = new FormData(event.currentTarget);

        const response = await fetch('http://localhost:3000/api/register', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                username: formData.get('username'),
                email: formData.get('email'),
                password: formData.get('password'),
            }),
        });

        if (!response.ok) {
            throw new Error(`Registration failed (HTTP ${response.status}).`);
        }
    }

    return (
        <div class="registration">
            <h1>Registration Page</h1>
            <p>Welcome to the registration page! Please fill out the form below to create an account.</p>
            <form onSubmit={handleSubmit}>
                <label for="username">Username:</label>
                <input type="text" id="username" name="username" required />
                <label for="email">Email:</label>
                <input type="email" id="email" name="email" required />
                <label for="password">Password:</label>
                <input type="password" id="password" name="password" required />
                <button type="submit">Register</button>
            </form>
        </div>
    );
}
