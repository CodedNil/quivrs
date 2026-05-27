use crate::shared::Region;

pub const fn labels(region: Region) -> &'static [&'static str] {
    match region {
        Region::Global => &[
            "global, worldwide, international, multinational, world, global scope, worldwide scope, international scope, worldwide reach, international reach",
            "globally, internationally, worldwide, transnational, planetary, world scale, global scale, international scale, worldwide audience, global audience",
        ],

        // Great Britain
        Region::UnitedKingdom => &[
            "United Kingdom, UK, U.K., Britain, Great Britain, British, Briton, Brits, UK-wide, nationwide Britain",
            "United Kingdom, Britain, British, Westminster, Whitehall, Downing Street, Westminster Palace, London SW1, UK national, British national",
        ],
        Region::England => &[
            "England, English, London, Manchester, Birmingham, Liverpool, Sheffield, Sunderland, Newcastle, Bristol",
            "Hackney, Golders Green, West Ham, Derbyshire, Dorset, Lancashire, Worcestershire, Herefordshire, Yorkshire, Kent",
            "Norfolk, Suffolk, Fordingbridge, River Waveney, River Wandle, River Lugg, Kew Gardens, Makerfield, Halifax, Wigan",
        ],
        Region::Scotland => &[
            "Scotland, Scottish, Scots, Edinburgh, Glasgow, Aberdeen, Dundee, Inverness, Stirling, Perth",
            "Holyrood, Arbroath, Peebles, Dunkeld, Highlands, A9, Perthshire, Fife, Ayrshire, Aberdeenshire",
        ],
        Region::Wales => &[
            "Wales, Welsh, Cymru, Cardiff, Swansea, Newport, Wrexham, Carmarthenshire, Pembrokeshire, Gwynedd",
            "Senedd, Fairwood, Eryri, Snowdonia, Bangor, Aberystwyth, Bridgend, Vale of Glamorgan, Flintshire, Powys",
        ],
        Region::Ireland => &[
            "Northern Ireland, Northern Irish, Ulster, Belfast, Derry, Londonderry, Stormont, Antrim, Armagh, County Down",
            "Ireland, Irish, Republic of Ireland, Dublin, Cork, Galway, Limerick, Donegal, County Kerry, Leinster",
        ],

        // North America
        Region::UnitedStates => &[
            "United States, USA, US, U.S., America, American, Washington DC, District of Columbia, Washington state, continental United States",
            "California, Californian, Los Angeles, San Francisco, San Diego, Sacramento, Silicon Valley, Oakland, San Jose, California state",
            "Texas, Texan, Austin, Houston, Dallas, San Antonio, Fort Worth, El Paso, Texas state, Lone Star State",
            "New York, New Yorker, New York City, NYC, Manhattan, Brooklyn, Albany, Buffalo, Queens, Long Island",
            "Florida, Floridian, Miami, Orlando, Tampa, Jacksonville, Tallahassee, Palm Beach, Florida state, Gulf Coast",
            "Washington state, Washingtonian, Seattle, Tacoma, Spokane, Olympia, Longview Washington, Puget Sound, Pacific Northwest, Washington",
            "Illinois, Chicago, Massachusetts, Boston, Georgia, Atlanta, Pennsylvania, Philadelphia, Arizona, Phoenix",
        ],
        Region::Canada => &[
            "Canada, Canadian, Ottawa, Ontario, Quebec, Alberta, British Columbia, Toronto, Montreal, Vancouver",
            "Calgary, Edmonton, Quebec City, Manitoba, Saskatchewan, Nova Scotia, New Brunswick, Newfoundland, Yukon, Nunavut",
        ],

        // Latin America and the Caribbean
        Region::LatinAmerica => &[
            "Mexico, Mexican, Mexico City, Guadalajara, Monterrey, Yucatan, Oaxaca, Tijuana, Cancun, Veracruz",
            "Brazil, Brazilian, Brasilia, Sao Paulo, Rio de Janeiro, Bahia, Amazonia, Pernambuco, Parana, Porto Alegre",
            "Argentina, Argentine, Argentinian, Buenos Aires, Cordoba, Mendoza, Rosario, Patagonia, Salta, La Plata",
            "Chile, Chilean, Santiago, Valparaiso, Atacama, Patagonia, Concepcion, Antofagasta, Andes, Araucania",
            "Colombia, Colombian, Bogota, Medellin, Cali, Cartagena, Barranquilla, Antioquia, Cundinamarca, Cauca",
            "Andean region, Andes, Peru, Peruvian, Lima, Cusco, Ecuador, Ecuadorian, Quito, Bolivia",
        ],
        Region::Caribbean => &[
            "Caribbean, Caribbean islands, Cuba, Cuban, Havana, Haiti, Haitian, Jamaica, Dominican Republic, Puerto Rico",
            "Barbados, Trinidad and Tobago, Bahamas, Grenada, Saint Lucia, Saint Vincent, Antigua, Dominica, Caribbean Sea, West Indies",
            "Cuba, Jamaica, Haiti, Dominican Republic, Puerto Rico, Trinidad, Barbados, Bahamas, Aruba, Curacao",
        ],

        // Europe
        Region::France => &[
            "France, French, Paris, Marseille, Lyon, Toulouse, Bordeaux, Lille, Nice, Strasbourg",
            "France, French Republic, French, Paris, Normandy, Brittany, Provence, Lyon, Bordeaux, Toulouse",
        ],
        Region::Germany => &[
            "Germany, German, Berlin, Munich, Hamburg, Frankfurt, Leipzig, Cologne, Bavaria, Saxony",
            "Germany, German, Deutschland, Berlin, Bavaria, Saxony, North Rhine-Westphalia, Stuttgart, Dresden, Hanover",
        ],
        Region::Italy => &[
            "Italy, Italian, Rome, Milan, Turin, Naples, Sicily, Tuscany, Bologna, Venice",
            "Italy, Italian, Italia, Rome, Lombardy, Sicily, Sardinia, Florence, Genoa, Palermo",
        ],
        Region::Iberia => &[
            "Spain, Spanish, Madrid, Barcelona, Valencia, Seville, Catalonia, Andalusia, Bilbao, Malaga",
            "Portugal, Portuguese, Lisbon, Porto, Algarve, Madeira, Azores, Coimbra, Braga, Sintra",
            "Iberia, Iberian, Iberian Peninsula, Spain, Portugal, Spanish, Portuguese, Madrid, Lisbon, Pyrenees",
        ],
        Region::Nordic => &[
            "Nordic, Nordic countries, Denmark, Danish, Copenhagen, Sweden, Swedish, Stockholm, Norway, Norwegian",
            "Finland, Finnish, Helsinki, Iceland, Icelandic, Reykjavik, Norway, Oslo, Sweden, Scandinavia",
        ],
        Region::WesternEurope => &[
            "Benelux, Netherlands, Dutch, Amsterdam, Rotterdam, Belgium, Belgian, Brussels, Luxembourg, Antwerp",
            "Western Europe, western European, continental Europe, European Union, EU, Brussels, eurozone, Schengen, western European Union, Europe",
        ],
        Region::CentralEurope => &[
            "Central Europe, central European, Austria, Austrian, Vienna, Salzburg, Switzerland, Swiss, Zurich, Geneva",
            "Poland, Polish, Warsaw, Krakow, Gdansk, Wroclaw, Poznan, Silesia, Lodz, Lublin",
            "Hungary, Hungarian, Budapest, Debrecen, Szeged, Pecs, Gyor, Balaton, Magyar, Danube",
            "Czechia, Czech, Prague, Brno, Ostrava, Bohemia, Moravia, Plzen, Olomouc, Czech Republic",
            "Slovakia, Slovak, Bratislava, Kosice, Presov, Zilina, Tatras, Trnava, Nitra, Slovak Republic",
        ],
        Region::Balkans => &[
            "Balkans, Balkan Peninsula, southeastern Europe, Serbia, Serbian, Belgrade, Croatia, Croatian, Zagreb, Bosnia",
            "Bulgaria, Bulgarian, Sofia, Slovenia, Slovenian, Ljubljana, Kosovo, Pristina, Albania, Tirana",
            "Greece, Greek, Athens, Thessaloniki, Macedonia, Peloponnese, Crete, Aegean, Hellenic, Balkans",
            "Bosnia and Herzegovina, Bosnian, Sarajevo, Montenegro, Montenegrin, Podgorica, North Macedonia, Skopje, Balkans, Balkan",
        ],
        Region::EasternEurope => &[
            "Ukraine, Ukrainian, Kyiv, Kharkiv, Odesa, Lviv, Donbas, Crimea, Dnipro, Zaporizhzhia",
            "Russia, Russian, Moscow, Saint Petersburg, Kremlin, Siberia, Kursk, Belgorod, Rostov, Volgograd",
            "Romania, Romanian, Bucharest, Transylvania, Cluj, Timisoara, Constanta, Iasi, Brasov, Wallachia",
            "Moldova, Moldovan, Chisinau, Balti, Transnistria, Orhei, Cahul, Comrat, Bessarabia, Moldavia",
            "Belarus, Belarusian, Minsk, Gomel, Brest, Vitebsk, Grodno, Mogilev, Polotsk, Belorussian",
            "Baltic, Baltic states, Estonia, Estonian, Tallinn, Latvia, Latvian, Riga, Lithuania, Vilnius",
        ],

        // Middle East and Africa
        Region::MiddleEastNorthAfrica => &[
            "Israel, Israeli, Jerusalem, Tel Aviv, Gaza, Palestine, Palestinian, West Bank, Ramallah, Haifa",
            "Iran, Iranian, Tehran, Isfahan, Shiraz, Tabriz, Mashhad, Qom, Persian, Persian Gulf",
            "Levant, Lebanon, Lebanese, Beirut, Tripoli Lebanon, Bekaa Valley, Syria, Damascus, Jordan, Amman",
            "Iraq, Iraqi, Baghdad, Basra, Mosul, Kurdistan, Erbil, Tigris, Euphrates, Mesopotamia",
            "Saudi Arabia, Saudi, Riyadh, Jeddah, Mecca, Medina, Neom, Hejaz, Najd, Saudi kingdom",
            "United Arab Emirates, UAE, Emirati, Dubai, Abu Dhabi, Sharjah, Ajman, Fujairah, Ras Al Khaimah, Emirates",
            "Qatar, Qatari, Doha, Kuwait, Kuwaiti, Kuwait City, Gulf, Persian Gulf, Arabian Peninsula, Gulf states",
            "Egypt, Egyptian, Cairo, Alexandria, Giza, Sinai, Nile, Luxor, Aswan, Suez",
            "Morocco, Moroccan, Rabat, Casablanca, Marrakesh, Tangier, Fez, Atlas, Agadir, Morocco kingdom",
            "Algeria, Algerian, Algiers, Oran, Constantine, Sahara, Kabylie, Annaba, Tlemcen, Algerian",
            "Tunisia, Tunisian, Tunis, Sfax, Carthage, Sousse, Kairouan, Djerba, Tunisia, Maghreb",
            "Libya, Libyan, Tripoli, Benghazi, Misrata, Sirte, Fezzan, Cyrenaica, North Africa, Maghreb",
        ],
        Region::SubSaharanAfrica => &[
            "Nigeria, Nigerian, Lagos, Abuja, Kano, Kaduna, Rivers, Ibadan, Enugu, Port Harcourt",
            "Ghana, Ghanaian, Accra, Kumasi, Tamale, Cape Coast, Ashanti, Volta, Tema, West Africa",
            "Senegal, Senegalese, Dakar, Saint-Louis, Casamance, Ivory Coast, Ivorian, Abidjan, West Africa, Sahel",
            "Kenya, Kenyan, Nairobi, Mombasa, Kisumu, Nakuru, Rift Valley, East Africa, Kenyan, Kenya",
            "Ethiopia, Ethiopian, Addis Ababa, Amhara, Tigray, Oromo, Oromia, Gondar, East Africa, Horn of Africa",
            "Uganda, Ugandan, Kampala, Entebbe, Jinja, Gulu, Lake Victoria, Buganda, East Africa, Uganda",
            "Tanzania, Tanzanian, Dar es Salaam, Zanzibar, Arusha, Dodoma, Serengeti, Kilimanjaro, East Africa, Tanzania",
            "Rwanda, Rwandan, Kigali, Burundi, Burundian, Bujumbura, Great Lakes, East Africa, Rwanda, Burundi",
            "South Africa, South African, Johannesburg, Pretoria, Cape Town, Durban, Gauteng, Western Cape, Limpopo, Soweto",
            "Zimbabwe, Zimbabwean, Harare, Bulawayo, Victoria Falls, Zambia, Zambian, Lusaka, Copperbelt, southern Africa",
            "Botswana, Botswanan, Gaborone, Kalahari, Okavango, Namibia, Namibian, Windhoek, southern Africa, southern African",
            "Congo, Democratic Republic of Congo, DRC, Congolese, Kinshasa, Lubumbashi, Kivu, Goma, central Africa, Congo River",
            "Angola, Angolan, Luanda, Cameroon, Cameroonian, Yaounde, Douala, Gabon, Libreville, central Africa",
        ],

        // Asia
        Region::India => &[
            "India, Indian, New Delhi, Delhi, Mumbai, Kolkata, Chennai, Bengaluru, Hyderabad, India subcontinent",
            "India, Indian, Gujarat, Maharashtra, Kerala, Tamil Nadu, West Bengal, Rajasthan, Punjab, Karnataka",
            "India, Indian, Bihar, Uttar Pradesh, Madhya Pradesh, Odisha, Andhra Pradesh, Telangana, Haryana, subcontinent",
            "Bangladesh, Bangladeshi, Dhaka, Chittagong, Sylhet, Khulna, Rajshahi, Cox's Bazar, Bengal, Padma",
            "Sri Lanka, Sri Lankan, Colombo, Kandy, Jaffna, Galle, Ceylon, Sinhalese, Tamil, Anuradhapura",
            "Maldives, Maldivian, Male, Addu, Hulhumale, Indian Ocean, atoll, Kaafu, Maafushi, Dhivehi",
            "Nepal, Nepalese, Kathmandu, Pokhara, Bhaktapur, Lalitpur, Himalayas, Kathmandu Valley, Himalayan, Nepal",
            "Bhutan, Bhutanese, Thimphu, Paro, Punakha, Himalayas, Himalayan kingdom, Druk, Wangdue, Bhutan",
        ],
        Region::WestAsia => &[
            "Pakistan, Pakistani, Islamabad, Karachi, Lahore, Punjab, Sindh, Peshawar, Balochistan, Rawalpindi",
            "Pakistan, Pakistani, Karachi, Lahore, Punjab, Sindh, Khyber Pakhtunkhwa, Balochistan, Kashmir, Islamabad",
            "Afghanistan, Afghan, Kabul, Kandahar, Herat, Jalalabad, Pashtun, Hindu Kush, Mazar-i-Sharif, Balkh",
            "Afghanistan, Afghan, Herat, Kunduz, Ghazni, Nangarhar, Helmand, Khyber Pass, Central Asia, West Asia",
        ],
        Region::China => &[
            "China, Chinese, Beijing, Shanghai, Shenzhen, Guangzhou, Wuhan, Sichuan, Guangdong, Zhejiang",
            "China, Chinese, People's Republic of China, Beijing, Shanghai, Guangdong, Sichuan, Zhejiang, Hubei, Xinjiang",
            "Hong Kong, Hongkonger, Kowloon, Macau, Macanese, Victoria Harbour, Hong Kong Island, New Territories, Lantau, Kowloon Peninsula",
        ],
        Region::Japan => &[
            "Japan, Japanese, Tokyo, Osaka, Kyoto, Hokkaido, Okinawa, Yokohama, Nagoya, Hiroshima",
            "Japan, Japanese, Tokyo, Kansai, Honshu, Hokkaido, Kyushu, Shikoku, Sapporo, Fukuoka",
        ],
        Region::Korea => &[
            "Korea, Korean, North Korea, South Korea, Korean Peninsula, Seoul, Pyongyang, Busan, Incheon, DPRK",
            "Korea, Korean, North Korean, North Korea, Democratic People's Republic of Korea, Pyongyang, Kaesong, Hamhung, Juche, Korean Peninsula",
            "Korea, Korean, Republic of Korea, South Korean, Seoul, Gyeonggi, Daejeon, Jeju, Busan, Incheon",
        ],
        Region::Taiwan => &[
            "Taiwan, Taiwanese, Taipei, Kaohsiung, Taichung, Tainan, Formosa, Hsinchu, Taiwan Strait, Taiwanese",
            "Taiwan, Taiwanese, Taipei, New Taipei, Taoyuan, Hsinchu, Kaohsiung, Taichung, Tainan, Republic of China",
        ],
        Region::SoutheastAsia => &[
            "Indonesia, Indonesian, Jakarta, Bali, Java, Sumatra, Sulawesi, Bandung, Surabaya, Borneo",
            "Vietnam, Vietnamese, Hanoi, Ho Chi Minh City, Saigon, Da Nang, Hue, Mekong, Haiphong, Vietnam",
            "Thailand, Thai, Bangkok, Chiang Mai, Phuket, Pattaya, Isan, Ayutthaya, Krabi, Thailand",
            "Philippines, Filipino, Manila, Cebu, Luzon, Mindanao, Davao, Quezon City, Visayas, Tagalog",
            "Malay Peninsula, Singapore, Singaporean, Malaysia, Malaysian, Kuala Lumpur, Penang, Johor, Sarawak, Sabah",
            "Mainland Southeast Asia, Myanmar, Burmese, Yangon, Cambodia, Cambodian, Phnom Penh, Laos, Lao, Vientiane",
            "Southeast Asia, Southeast Asian, ASEAN, South China Sea, Mekong, Indochina, Malacca Strait, maritime Southeast Asia, mainland Southeast Asia, ASEAN states",
        ],
        Region::CentralAsia => &[
            "Kazakhstan, Kazakh, Astana, Almaty, Karaganda, Shymkent, Aktobe, Caspian, steppe, Central Asia",
            "Uzbekistan, Uzbek, Tashkent, Samarkand, Bukhara, Khiva, Fergana, Nukus, Andijan, Central Asia",
            "Kyrgyzstan, Kyrgyz, Bishkek, Osh, Issyk-Kul, Naryn, Jalal-Abad, Tien Shan, Central Asia, Kyrgyz Republic",
            "Tajikistan, Tajik, Dushanbe, Pamir, Khujand, Khorugh, Fann Mountains, Central Asia, Gorno-Badakhshan, Tajikistani",
            "Turkmenistan, Turkmen, Ashgabat, Caspian Sea, Mary, Turkmenbashi, Karakum, Central Asia, Balkan Region, Turkmenistani",
            "Mongolia, Mongolian, Ulaanbaatar, Gobi, Steppe, Erdenet, Darkhan, Khovd, Mongol, Central Mongolia",
        ],

        // Oceania
        Region::Oceania => &[
            "Australia, Australian, Canberra, Sydney, Melbourne, Perth, Brisbane, Queensland, Tasmania, Western Australia",
            "New Zealand, New Zealander, Wellington, Auckland, Christchurch, Queenstown, North Island, South Island, Aotearoa, Kiwi",
            "Pacific islands, Fiji, Fijian, Samoa, Samoan, Tonga, Tongan, Papua New Guinea, Melanesia, Polynesia",
            "Oceania, Oceanian, Australasia, Pacific, South Pacific, Australia, New Zealand, Pacific Ocean, Pacific region, Australasian",
        ],
    }
}
