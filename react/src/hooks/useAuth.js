import { connectWallet, disconnectWallet } from '../services/connectWallet';
import { useState, useEffect } from 'react';
import toast from 'react-hot-toast';

import axios from 'axios';

export const useAuth = () => {
  // 1. Initialize isLoading to true so it "waits" for the useEffect
  const [isLoading, setIsLoading] = useState(true);
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  useEffect(() => {
    const verify = async () => {
      try {
        // The endpoint should check the cookie/session on the server
        const res = await axios.post('/api/user/verify');
        setIsAuthenticated(res.status === 200);
      } catch (error) {
        console.error("Verification failed:", error);
        setIsAuthenticated(false);
      } finally {
        // 2. Always turn off loading, whether success or failure
        setIsLoading(false);
      }
    };

    verify();
  }, []); // Empty dependency array ensures this runs exactly once on mount

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
        toast.error("No Web3 wallet detected!");
      } else {
        toast.error("Failed to connect wallet.");
      }
    } finally {
      setIsLoading(false);
    }
  };

  const logout = async () => {
    try {
      await disconnectWallet();
      // If your API uses cookies, you might need an axios call here to clear them
      localStorage.removeItem('sessionToken');
      setIsAuthenticated(false);
      toast.success("Disconnected successfully.");
    } catch (error) {
      toast.error("An error occurred while disconnecting.");
    }
  };

  return { isLoading, isAuthenticated, login, logout };
};
