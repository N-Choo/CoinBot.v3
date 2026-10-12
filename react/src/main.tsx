import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom'; // 1. Import BrowserRouter
import './styles/tokens.css'
import './styles/base.css'
import './index.css'
import AppRoutes from './App';
import axios from 'axios';


import { loadConfig } from './services/transaction';

loadConfig()

// Set the global base URL for all Axios requests
axios.defaults.withCredentials = true;

axios.defaults.headers.common['Content-Type'] = 'application/json';
// Sane default; slow endpoints (e.g. deposit intake) set a longer per-request timeout.
axios.defaults.timeout = 5000;

// On session expiry, bounce protected pages back to the landing page.
let redirecting = false;
axios.interceptors.response.use(
  res => res,
  err => {
    if (err?.response?.status === 401 && !redirecting) {
      const path = window.location.pathname;
      if (path.startsWith('/dashboard') || path.startsWith('/trading')) {
        redirecting = true;
        window.location.assign('/');
      }
    }
    return Promise.reject(err);
  },
);

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {/* 2. Wrap your Routes inside BrowserRouter */}
    <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
      <AppRoutes />
    </BrowserRouter>
  </React.StrictMode>
);
