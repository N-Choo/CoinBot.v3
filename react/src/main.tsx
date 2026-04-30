import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom'; // 1. Import BrowserRouter
import './styles/main.css'; // Your global styles
import AppRoutes from './App';
import axios from 'axios';


// Set the global base URL for all Axios requests
axios.defaults.baseURL = 'http://localhost:3000';

// Optional: Set global headers or timeouts
axios.defaults.headers.common['Content-Type'] = 'application/json';
axios.defaults.timeout = 5000;

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {/* 2. Wrap your Routes inside BrowserRouter */}
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  </React.StrictMode>
);
