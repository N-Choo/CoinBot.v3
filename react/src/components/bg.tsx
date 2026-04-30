import React, { ReactNode } from 'react';
import '../styles/bg.css'; // <-- 1. Import the CSS file here

interface BackgroundProps {
  children: ReactNode;
}

const Background = ({ children }: BackgroundProps) => {
  return (
    <div className="app-background">
      {/* 2. The <style> tag was removed from here */}

      {/* Ambient Glowing Orbs */}
      <div className="glow-blob blob-top-right"></div>
      <div className="glow-blob blob-bottom-left"></div>

      {/* Main Content Layer */}
      <div className="content-layer">
        {children}
      </div>
    </div>
  );
};

export default Background;
