import { connectWallet } from '../services/connectWallet';
import { useState, useEffect } from 'react';
import toast from 'react-hot-toast';

export const useAuth = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  useEffect(() => {
    const token = localStorage.getItem('sessionToken');
    if (token) {
      setIsAuthenticated(true);
    }
  }, []);

  const login = async () => {
    setIsLoading(true);

    try {
      const success = await connectWallet();

      if (success) {
        setIsAuthenticated(true);
        toast.success("Wallet connected successfully!");
      }
    } catch (error) {
      if (error.message === "NO_WALLET") {
        toast.error("No Web3 wallet detected! Please install MetaMask, Rabby, or Coinbase Wallet.");
      } else {
        console.error("Wallet connection failed:", error);
        toast.error("Failed to connect wallet. Please try again.");
      }
    } finally {
      setIsLoading(false);
    }
  };

  const logout = () => {
    localStorage.removeItem('sessionToken');
    setIsAuthenticated(false);
    toast.success("Disconnected successfully.");
  };

  return { isLoading, isAuthenticated, login, logout };
};
