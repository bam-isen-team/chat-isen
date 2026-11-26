import type { TargetedSubmitEvent } from 'preact';
import { useState } from 'preact/hooks';
import './Registration.css';

interface RegistrationResponse {
    access_token: string;
    user: {
        id: string;
        username: string;
        email: string;
        display_name: string;
    };
}

interface RegistrationError {
    error: string;
}

export function Registration() {
    const [errorMessage, setErrorMessage] = useState('');
    const [successMessage, setSuccessMessage] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);

    async function handleSubmit(event: TargetedSubmitEvent<HTMLFormElement>) {
        event.preventDefault();
        const formData = new FormData(event.currentTarget);

        setErrorMessage('');
        setSuccessMessage('');
        setIsSubmitting(true);

        try {
            const response = await fetch('http://localhost:3000/auth/register', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    username: formData.get('username')?.toString() ?? '',
                    email: formData.get('email')?.toString() ?? '',
                    password: formData.get('password')?.toString() ?? '',
                }),
            });

            if (response.status === 201) {
                const result: RegistrationResponse = await response.json();
                setSuccessMessage(`Registration successful. Welcome, ${result.user.display_name}!`);
                return;
            }

            if (response.status === 400) {
                const result: RegistrationError = await response.json();
                setErrorMessage(result.error);
                return;
            }

            if (response.status === 409) {
                setErrorMessage('An account with this username or email already exists.');
                return;
            }

            setErrorMessage(`Registration failed (HTTP ${response.status}).`);
        } catch (error) {
            setErrorMessage(
                error instanceof Error ? `Unable to register: ${error.message}` : 'Unable to register.',
            );
        } finally {
            setIsSubmitting(false);
        }
    }

    return (
        <div class="registration">
            <h1>Registration Page</h1>
            <p>Welcome to the registration page! Please fill out the form below to create an account.</p>
            {errorMessage && <p role="alert">{errorMessage}</p>}
            {successMessage && <p role="status">{successMessage}</p>}
            <form onSubmit={handleSubmit}>
                <label for="username">Username:</label>
                <input type="text" id="username" name="username" required />
                <label for="email">Email:</label>
                <input type="email" id="email" name="email" required />
                <label for="password">Password:</label>
                <input type="password" id="password" name="password" required />
                <button type="submit" disabled={isSubmitting}>
                    {isSubmitting ? 'Registering...' : 'Register'}
                </button>
            </form>
        </div>
    );
}
