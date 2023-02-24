import React from 'react';
import logo from './logo.svg';
import './App.css';

function App() {
    return (
        <div className="App">
            <header className="App-header">
                <img src={logo} className="App-logo" alt="logo"/>
                <p>
                    Edit <code>src/App.tsx</code> and save to reload.
                </p>
                <a
                    className="App-link"
                    href="https://reactjs.org"
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    Learn React
                </a>
            </header>
            <br/>
            <h1>Contact</h1>
            <form data-static-form-name="contact">
                <div>
                    <label>Name<input type="text" name="name"/></label>
                </div>
                <div>
                    <label>Email<input type="email" name="email"/></label>
                </div>
                <div>
                    <label>Message<textarea name="message"></textarea></label>
                </div>
                <button type="submit">Send!</button>
            </form>
        </div>
    );
}

export default App;
