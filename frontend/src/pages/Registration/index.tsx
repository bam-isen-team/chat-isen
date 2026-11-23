import './style.css';



export function Registration() {
    return (
        <div class="registration">
            <h1>Registration Page</h1>
            <p>Welcome to the registration page! Please fill out the form below to create an account.</p>
            <form>
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