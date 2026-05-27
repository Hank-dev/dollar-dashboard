import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import { TopNav } from "@/components/TopNav";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  variable: "--font-jetbrains-mono",
  subsets: ["latin"],
  display: "swap",
});

export const metadata: Metadata = {
  title: "Market Monitor",
  description: "Quantitative dashboards for crypto, macro, and AI.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${inter.variable} ${jetbrainsMono.variable} h-full antialiased`}
    >
      <body className="min-h-screen bg-[var(--bg-base)] text-[var(--text-primary)]">
        <TopNav />
        {children}
      </body>
    </html>
  );
}
