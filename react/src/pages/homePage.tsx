import '../styles/homepage.css'
import HeroSection from '../components/landing/HeroSection'
import FeaturesSection from '../components/landing/FeaturesSection'
import FooterSection from '../components/landing/FooterSection'

export default function Landing() {
  return (
    <div className="landing">
      <HeroSection />
      <FeaturesSection />
      <FooterSection />
    </div>
  )
}
