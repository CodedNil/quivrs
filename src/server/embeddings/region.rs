use crate::shared::Region;

pub const fn labels(region: Region) -> &'static [&'static str] {
    match region {
        Region::Global => &[
            "Global scale: worldwide scope, international reach, transnational markets, cross-border coverage, multinational systems, universal themes, world economy, global supply chains, international trade networks, multilateral treaties, worldwide policy, universal scientific phenomena, atmospheric science, commodity markets, international economic turmoil, global trade tariffs.",
            "Location-independent digital culture: internet culture, software deployment, open-source programming, developer tooling, workspace managers, compiler engineering, register allocation, runtime optimization, web applications, prompt workflows, router configuration, wireless signal advice, consumer applications, camera apps, smart devices, gaming hardware, tablet hardware, consumer electronics updates.",
            "Location-independent consumer and media guides: streaming television guides, online viewing options, drama series releases, mystery thrillers, entertainment reviews, consumer product launches, brand news, shopping guides, deal roundups, product comparisons, general lifestyle advice.",
            "General consumer product coverage: retail brand launches, sale roundups, product reviews, shopping advice, consumer electronics, gaming deals, appliance deals, wearable devices, lifestyle bargains, market promotions.",
            "Universal local-interest topics: park upkeep, community cleanup appeals, heritage appeals, general gardening, tomato planting, agricultural botany, global flora, worldwide internet trends, viral news.",
        ],

        // Great Britain
        Region::UnitedKingdom => &[
            "United Kingdom: UK, U.K., Great Britain, Britain, British, Briton, British national scope, nationwide UK territory, Westminster administration, Whitehall departments, Downing Street executive, Parliament legislation, UK sovereign governance, British national infrastructure, nationalised utilities, water companies, energy companies, birth rates across England and Wales.",
            "UK consumer economy: sterling currency, pound sterling finance, GBP banknotes, British pound pricing, city centre cafe prices, high street prices, domestic household energy bills, national utility regulators, British retail costs, grocery prices, food production investment, supplier agreements, British grocery supply chains, cost-of-living pressures.",
        ],
        Region::England => &[
            "England: English territory, Anglo geography, English municipalities, English local government councils, English regional constituencies, London metro, Manchester urban area, Birmingham district, Liverpool region, Yorkshire counties, Midlands, English shires, southern counties, northern English towns, English inland waterways, English environmental zones, regional preservation sites, community parklands, Bournemouth Beach, Cardingmill Valley, Shrewsbury, River Waveney, National Trust property, English coastlines, English local attractions, English towns, English beaches, coastal incidents, local incidents, heritage appeals, local conservation appeals, countryside cleanup appeals, bank holiday weekend, rural littering, heritage landmarks, England domestic bills, local marketplace transactions.",
        ],
        Region::Scotland => &[
            "Scotland: Scottish nation, Scots territory, Scottish highlands, Scottish lowlands, Scottish regional authorities, Scottish island chains, Holyrood administration, Scottish parliament, Edinburgh metro, Glasgow urban area, Aberdeen district, Scottish municipal councils, historic Scottish burghs, local Scottish waterways, Scottish pound notes, local Scottish retail transactions.",
        ],
        Region::Wales => &[
            "Wales: Welsh nation, Cymru territory, Welsh valleys, Welsh regional authorities, local Welsh municipal districts, Senedd administration, Cardiff metro, Swansea urban area, Newport district, Welsh county councils, Welsh coastal zones, Welsh beaches, Welsh seaside incidents, Welsh shoreline litter, historic Welsh principalities, Welsh regional transactions, local Welsh community funding.",
        ],
        Region::Ireland => &[
            "Ireland: Republic of Ireland, Eire, Northern Ireland, Northern Irish territory, Ulster province, Stormont assembly, Belfast metro, Derry district, Northern Irish municipal boundaries, Dublin metro, Cork region, Irish national governance, Irish provincial geography, Irish counties, Ulster Wildlife, Irish countryside, native woodland restoration, peatland recovery, habitat restoration, biodiversity projects.",
        ],

        // North America
        Region::UnitedStates => &[
            "United States: USA, US, U.S., America, American, Washington DC executive, federal governance, American nationwide scope, California state, Texas territory, New York geography, Florida peninsula, Washington state, Midwest, Pacific Northwest, New England, major American metropolitan areas, US state legislatures, American municipal counties, domestic US interstate commerce, American regional infrastructure.",
            "US public institutions and commerce: United States dollar currency, USD bills, $ symbol, dollar pricing, American retail ads, online shopping deals, holiday sales, Memorial Day, White House, attorney general, federal court prosecutions, federal prosecutors, US federal agencies, American tech headquarters, Silicon Valley corporate news, corporate headquarters, venture capital, business regulation.",
        ],
        Region::Canada => &[
            "Canada: Canadian nation, Canadian provinces, Canadian territories, Ottawa federal executive, Canadian nationwide scope, Ontario geography, Quebec territory, Alberta region, British Columbia coast, Toronto metro, Montreal urban area, Vancouver district, Canadian municipal sectors, Canadian dollar financing, CAD market, domestic Canada pricing.",
        ],

        // Latin America and the Caribbean
        Region::LatinAmerica => &[
            "Latin America: Central America, South America, Hispanic regions, Andean zone, Amazonian basin, Mesoamerican geography, Mexico, Brazil, Argentina, Chile, Colombia, Peru, sovereign Latin American governance, South American municipal sectors, regional metropolitan areas, Mexico City, Sao Paulo, Buenos Aires, Bogota, Santiago.",
        ],
        Region::Caribbean => &[
            "Caribbean basin: West Indies, Caribbean archipelago, tropical island states, Caribbean sea territories, Cuba, Haiti, Jamaica, Dominican Republic, Puerto Rico, Bahamas, Barbados, island nation governance, maritime territorial politics.",
        ],

        // Europe
        Region::France => &[
            "France: French republic, French territory, Parisian metro, Western European geography, French national governance, hexagone regions, Roland Garros tournament, French Open sporting venues, Paris courts, domestic French athletic events.",
        ],
        Region::Germany => &[
            "Germany: German, Deutschland, Berlin, Munich, Hamburg, Frankfurt, Leipzig, Cologne, Bavaria, Saxony, domestic Germany operations, North Rhine-Westphalia, Stuttgart, Dresden, Hanover, and other German states.",
        ],
        Region::Italy => &[
            "Italy: Italian, Italia, Rome, Milan, Turin, Naples, Sicily, Tuscany, Bologna, Venice, domestic Italian operations, Lombardy, Sardinia, Florence, Genoa, Palermo, and other Italian regions.",
        ],
        Region::Iberia => &[
            "Iberia: Spain, Spanish, Madrid, Barcelona, Valencia, Seville, Catalonia, Andalusia, Bilbao, Malaga, Spain national news, Portugal, Portuguese, Lisbon, Porto, Algarve, Madeira, Azores, Coimbra, Braga, Sintra, Portugal national news, Iberian Peninsula, Pyrenees, and cross-border Iberian coverage.",
        ],
        Region::Nordic => &[
            "Nordic: Nordic countries, Denmark, Danish, Copenhagen, Sweden, Swedish, Stockholm, Norway, Norwegian, Nordic regional news, Finland, Finnish, Helsinki, Iceland, Icelandic, Reykjavik, Norway, Oslo, Scandinavia, and northern European coverage.",
        ],
        Region::WesternEurope => &[
            "Western Europe: Benelux, Netherlands, Dutch, Amsterdam, Rotterdam, Belgium, Belgian, Brussels, Luxembourg, Antwerp, Benelux national news, continental Europe, European Union, EU, Brussels, eurozone, Schengen, European institutions, and pan-European coverage.",
        ],
        Region::CentralEurope => &[
            "Central Europe: Austria, Austrian, Vienna, Salzburg, Switzerland, Swiss, Zurich, Geneva, Poland, Polish, Warsaw, Krakow, Gdansk, Wroclaw, Poznan, Silesia, Hungary, Hungarian, Budapest, Czechia, Czech, Prague, Slovakia, Slovak, Bratislava, Central European regional news.",
        ],
        Region::Balkans => &[
            "Balkans: Balkan Peninsula, southeastern Europe, Serbia, Serbian, Belgrade, Croatia, Croatian, Zagreb, Bosnia, Bulgaria, Bulgarian, Sofia, Slovenia, Slovenian, Ljubljana, Kosovo, Pristina, Albania, Tirana, Greece, Greek, Athens, Thessaloniki, Macedonia, Peloponnese, Crete, Aegean, Hellenic, Bosnia and Herzegovina, Bosnian, Sarajevo, Montenegro, North Macedonia, Skopje.",
        ],
        Region::EasternEurope => &[
            "Eastern Europe: Ukraine, Ukrainian, Kyiv, Kharkiv, Odesa, Lviv, Donbas, Crimea, Russia, Russian, Moscow, Saint Petersburg, Kremlin, Siberia, Kursk, Belgorod, Romania, Romanian, Bucharest, Transylvania, Moldova, Moldovan, Chisinau, Belarus, Belarusian, Minsk, Baltic states, Estonia, Tallinn, Latvia, Riga, Lithuania, Vilnius.",
        ],

        // Middle East and Africa
        Region::MiddleEastNorthAfrica => &[
            "Middle East & North Africa: Israel, Israeli, Jerusalem, Tel Aviv, Gaza, Palestine, Palestinian, West Bank, Iran, Iranian, Tehran, Persian Gulf, Levant, Lebanon, Lebanese, Beirut, Syria, Damascus, Jordan, Amman, Iraq, Iraqi, Baghdad, Saudi Arabia, Saudi, Riyadh, United Arab Emirates, UAE, Emirati, Dubai, Abu Dhabi, Qatar, Qatari, Doha, Kuwait, Egypt, Egyptian, Cairo, Morocco, Moroccan, Casablanca, Algeria, Algerian, Algiers, Tunisia, Tunisian, Tunis, Libya, Libyan, Tripoli.",
        ],
        Region::SubSaharanAfrica => &[
            "Sub-Saharan Africa: Nigeria, Nigerian, Lagos, Ghana, Ghanaian, Accra, Senegal, West Africa, Sahel, Kenya, Kenyan, Nairobi, East Africa, Ethiopia, Ethiopian, Addis Ababa, Uganda, Ugandan, Kampala, Tanzania, Tanzanian, Dar es Salaam, Rwanda, Kigali, South Africa, South African, Johannesburg, Cape Town, Zimbabwe, Harare, Zambia, Lusaka, Botswana, Central Africa, Congo, DRC, Kinshasa, Angola, Luanda, Cameroon.",
        ],

        // Asia
        Region::India => &[
            "India: Indian subcontinent, New Delhi, Delhi, Mumbai, Kolkata, Chennai, Bengaluru, Hyderabad, Indian national news, Gujarat, Maharashtra, Kerala, Tamil Nadu, West Bengal, Rajasthan, Punjab, Karnataka, Bangladesh, Bangladeshi, Dhaka, Sri Lanka, Sri Lankan, Colombo, Maldives, Nepal, Kathmandu, Bhutan.",
        ],
        Region::WestAsia => &[
            "West Asia: Pakistan, Pakistani, Islamabad, Karachi, Lahore, Punjab, Sindh, Peshawar, Balochistan, Rawalpindi, Afghanistan, Afghan, Kabul, Kandahar, Herat, Hindu Kush, Central Asia regional coverage.",
        ],
        Region::China => &[
            "China: Chinese, People's Republic of China, Beijing, Shanghai, Shenzhen, Guangzhou, Wuhan, Sichuan, Guangdong, Zhejiang, Xinjiang, Hong Kong, Hongkonger, Kowloon, Macau, Macanese, Victoria Harbour, New Territories.",
        ],
        Region::Japan => &[
            "Japan: Japanese, Tokyo, Osaka, Kyoto, Hokkaido, Okinawa, Yokohama, Nagoya, Hiroshima, Kansai, Honshu, Kyushu, Shikoku, Sapporo, Fukuoka.",
        ],
        Region::Korea => &[
            "Korea: Korean Peninsula, North Korea, South Korea, Seoul, Pyongyang, Busan, Incheon, DPRK, Republic of Korea, South Korean, Gyeonggi, Daejeon, Jeju.",
        ],
        Region::Taiwan => &[
            "Taiwan: Taiwanese, Taipei, Kaohsiung, Taichung, Tainan, Formosa, Hsinchu, Taiwan Strait, Republic of China, Taiwanese politics, Taiwanese business.",
        ],
        Region::SoutheastAsia => &[
            "Southeast Asia: ASEAN, South China Sea, Mekong, Indochina, Malacca Strait, Indonesia, Indonesian, Jakarta, Bali, Vietnam, Vietnamese, Hanoi, Ho Chi Minh City, Saigon, Thailand, Thai, Bangkok, Philippines, Filipino, Manila, Singapore, Singaporean, Malaysia, Malaysian, Kuala Lumpur, Myanmar, Burmese, Cambodia, Laos.",
        ],
        Region::CentralAsia => &[
            "Central Asia: Kazakhstan, Kazakh, Astana, Almaty, Uzbekistan, Uzbek, Tashkent, Kyrgyzstan, Kyrgyz, Bishkek, Tajikistan, Tajik, Dushanbe, Turkmenistan, Turkmen, Ashgabat, Mongolia, Mongolian, Ulaanbaatar, Gobi desert, Central Asian steppe landscape.",
        ],

        // Oceania
        Region::Oceania => &[
            "Oceania: Australasia, Pacific region, South Pacific, Australia, Australian, Canberra, Sydney, Melbourne, Perth, Brisbane, Queensland, New Zealand, New Zealander, Wellington, Auckland, Christchurch, Aotearoa, Kiwi, Pacific islands, Fiji, Samoa, Tonga, Papua New Guinea, Australian federal matters, defence sites, environmental contamination, national legal cases, A$ pricing.",
        ],
    }
}
