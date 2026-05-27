use crate::shared::Region;

pub const fn labels(region: Region) -> &'static [&'static str] {
    match region {
        Region::Global => &[
            "Global, worldwide, international, cross-border, transnational, multinational, world scale, global scope, worldwide reach, international reach.",
            "A story with no meaningful local geography, no country-specific angle, or a topic that applies broadly everywhere.",
            "General news, consumer news, product launches, software updates, entertainment releases, or online culture that is not tied to one place.",
            "A company, platform, or internet service story that could apply in many countries and does not depend on a specific city or region.",
            "World economy, global supply chains, international markets, worldwide policy, or multinational institutions.",
            "A story about a person, product, app, device, or idea where the location is incidental or absent.",
            "Internet-wide, online, digital, virtual, remote, borderless, and location-agnostic coverage.",
            "A topic that is broadly relevant but not anchored to a local, national, or regional place name.",
        ],

        // Great Britain
        Region::UnitedKingdom => &[
            "United Kingdom, UK, U.K., Britain, Great Britain, British, Briton, UK-wide, nationwide Britain, British national story.",
            "UK politics, Westminster, Whitehall, Downing Street, Parliament, devolved nations, or a story framed as British national news.",
        ],
        Region::England => &[
            "England, English, London, Manchester, Birmingham, Liverpool, Sheffield, Sunderland, Newcastle, Bristol, or an English local news story.",
            "English counties, towns, and districts such as Hackney, Derbyshire, Dorset, Lancashire, Yorkshire, Kent, Norfolk, Suffolk, and similar places.",
            "Local English places, rivers, councils, and constituencies such as Fordingbridge, the River Waveney, the River Wandle, the River Lugg, Kew Gardens, Makerfield, Halifax, and Wigan.",
        ],
        Region::Scotland => &[
            "Scotland, Scottish, Scots, Edinburgh, Glasgow, Aberdeen, Dundee, Inverness, Stirling, Perth, or a Scottish national story.",
            "Holyrood, Scottish councils, Highlands, Perthshire, Fife, Ayrshire, Aberdeenshire, and towns like Arbroath, Peebles, and Dunkeld.",
        ],
        Region::Wales => &[
            "Wales, Welsh, Cymru, Cardiff, Swansea, Newport, Wrexham, Carmarthenshire, Pembrokeshire, Gwynedd, or a Welsh national story.",
            "Senedd, Welsh councils, Eryri, Snowdonia, Bangor, Aberystwyth, Bridgend, Vale of Glamorgan, Flintshire, and Powys.",
        ],
        Region::Ireland => &[
            "Northern Ireland, Northern Irish, Ulster, Belfast, Derry, Londonderry, Stormont, Antrim, Armagh, or County Down.",
            "Ireland, Irish, Republic of Ireland, Dublin, Cork, Galway, Limerick, Donegal, County Kerry, Leinster, or an Irish national story.",
        ],

        // North America
        Region::UnitedStates => &[
            "United States, USA, US, U.S., America, American, Washington DC, federal U.S. politics, or a national American story.",
            "California, Californian, Los Angeles, San Francisco, San Diego, Sacramento, Silicon Valley, Oakland, San Jose, or California state news.",
            "Texas, Texan, Austin, Houston, Dallas, San Antonio, Fort Worth, El Paso, or Texas state news.",
            "New York, New Yorker, New York City, NYC, Manhattan, Brooklyn, Albany, Buffalo, Queens, Long Island, or New York state news.",
            "Florida, Floridian, Miami, Orlando, Tampa, Jacksonville, Tallahassee, Palm Beach, or Florida state news.",
            "Washington state, Washingtonian, Seattle, Tacoma, Spokane, Olympia, Longview Washington, Puget Sound, or Pacific Northwest news.",
            "Illinois, Chicago, Massachusetts, Boston, Georgia, Atlanta, Pennsylvania, Philadelphia, Arizona, Phoenix, or other major U.S. metros.",
        ],
        Region::Canada => &[
            "Canada, Canadian, Ottawa, Ontario, Quebec, Alberta, British Columbia, Toronto, Montreal, Vancouver, or a Canadian national story.",
            "Calgary, Edmonton, Quebec City, Manitoba, Saskatchewan, Nova Scotia, New Brunswick, Newfoundland, Yukon, Nunavut, and other Canadian provinces and territories.",
        ],

        // Latin America and the Caribbean
        Region::LatinAmerica => &[
            "Mexico, Mexican, Mexico City, Guadalajara, Monterrey, Yucatan, Oaxaca, Tijuana, Cancun, Veracruz, or Mexico national news.",
            "Brazil, Brazilian, Brasilia, Sao Paulo, Rio de Janeiro, Bahia, Amazonia, Pernambuco, Parana, Porto Alegre, or Brazil national news.",
            "Argentina, Argentine, Argentinian, Buenos Aires, Cordoba, Mendoza, Rosario, Patagonia, Salta, La Plata, or Argentina national news.",
            "Chile, Chilean, Santiago, Valparaiso, Atacama, Patagonia, Concepcion, Antofagasta, Andes, Araucania, or Chile national news.",
            "Colombia, Colombian, Bogota, Medellin, Cali, Cartagena, Barranquilla, Antioquia, Cundinamarca, Cauca, or Colombia national news.",
            "Andean region, Andes, Peru, Peruvian, Lima, Cusco, Ecuador, Ecuadorian, Quito, Bolivia, or other South American coverage.",
        ],
        Region::Caribbean => &[
            "Caribbean, Caribbean islands, Cuba, Cuban, Havana, Haiti, Haitian, Jamaica, Dominican Republic, Puerto Rico, or Caribbean regional news.",
            "Barbados, Trinidad and Tobago, Bahamas, Grenada, Saint Lucia, Saint Vincent, Antigua, Dominica, Caribbean Sea, West Indies, and nearby islands.",
            "Cuba, Jamaica, Haiti, Dominican Republic, Puerto Rico, Trinidad, Barbados, Bahamas, Aruba, Curacao, or island politics and society.",
        ],

        // Europe
        Region::France => &[
            "France, French, Paris, Marseille, Lyon, Toulouse, Bordeaux, Lille, Nice, Strasbourg, or France national news.",
            "France, French Republic, Paris, Normandy, Brittany, Provence, Lyon, Bordeaux, Toulouse, and other French regions.",
        ],
        Region::Germany => &[
            "Germany, German, Berlin, Munich, Hamburg, Frankfurt, Leipzig, Cologne, Bavaria, Saxony, or Germany national news.",
            "Germany, Deutschland, Berlin, Bavaria, Saxony, North Rhine-Westphalia, Stuttgart, Dresden, Hanover, and other German states.",
        ],
        Region::Italy => &[
            "Italy, Italian, Rome, Milan, Turin, Naples, Sicily, Tuscany, Bologna, Venice, or Italy national news.",
            "Italy, Italia, Rome, Lombardy, Sicily, Sardinia, Florence, Genoa, Palermo, and other Italian regions.",
        ],
        Region::Iberia => &[
            "Spain, Spanish, Madrid, Barcelona, Valencia, Seville, Catalonia, Andalusia, Bilbao, Malaga, or Spain national news.",
            "Portugal, Portuguese, Lisbon, Porto, Algarve, Madeira, Azores, Coimbra, Braga, Sintra, or Portugal national news.",
            "Iberia, Iberian Peninsula, Spain, Portugal, Madrid, Lisbon, Pyrenees, and cross-border Iberian coverage.",
        ],
        Region::Nordic => &[
            "Nordic, Nordic countries, Denmark, Danish, Copenhagen, Sweden, Swedish, Stockholm, Norway, Norwegian, or Nordic regional news.",
            "Finland, Finnish, Helsinki, Iceland, Icelandic, Reykjavik, Norway, Oslo, Sweden, Scandinavia, and northern European coverage.",
        ],
        Region::WesternEurope => &[
            "Benelux, Netherlands, Dutch, Amsterdam, Rotterdam, Belgium, Belgian, Brussels, Luxembourg, Antwerp, or Benelux national news.",
            "Western Europe, continental Europe, European Union, EU, Brussels, eurozone, Schengen, European institutions, and pan-European coverage.",
        ],
        Region::CentralEurope => &[
            "Central Europe, Austria, Austrian, Vienna, Salzburg, Switzerland, Swiss, Zurich, Geneva, or Central European regional news.",
            "Poland, Polish, Warsaw, Krakow, Gdansk, Wroclaw, Poznan, Silesia, Lodz, Lublin, and Polish national news.",
            "Hungary, Hungarian, Budapest, Debrecen, Szeged, Pecs, Gyor, Balaton, Magyar, Danube, and Hungarian national news.",
            "Czechia, Czech, Prague, Brno, Ostrava, Bohemia, Moravia, Plzen, Olomouc, Czech Republic, and Czech national news.",
            "Slovakia, Slovak, Bratislava, Kosice, Presov, Zilina, Tatras, Trnava, Nitra, Slovak Republic, and Slovak national news.",
        ],
        Region::Balkans => &[
            "Balkans, Balkan Peninsula, southeastern Europe, Serbia, Serbian, Belgrade, Croatia, Croatian, Zagreb, Bosnia, or Balkan regional news.",
            "Bulgaria, Bulgarian, Sofia, Slovenia, Slovenian, Ljubljana, Kosovo, Pristina, Albania, Tirana, and nearby Balkan states.",
            "Greece, Greek, Athens, Thessaloniki, Macedonia, Peloponnese, Crete, Aegean, Hellenic, Balkans, or Greek national news.",
            "Bosnia and Herzegovina, Bosnian, Sarajevo, Montenegro, Montenegrin, Podgorica, North Macedonia, Skopje, Balkans, and South East Europe.",
        ],
        Region::EasternEurope => &[
            "Ukraine, Ukrainian, Kyiv, Kharkiv, Odesa, Lviv, Donbas, Crimea, Dnipro, Zaporizhzhia, or Ukraine national news.",
            "Russia, Russian, Moscow, Saint Petersburg, Kremlin, Siberia, Kursk, Belgorod, Rostov, Volgograd, or Russia national news.",
            "Romania, Romanian, Bucharest, Transylvania, Cluj, Timisoara, Constanta, Iasi, Brasov, Wallachia, or Romania national news.",
            "Moldova, Moldovan, Chisinau, Balti, Transnistria, Orhei, Cahul, Comrat, Bessarabia, Moldavia, or Moldova national news.",
            "Belarus, Belarusian, Minsk, Gomel, Brest, Vitebsk, Grodno, Mogilev, Polotsk, Belorussian, or Belarus national news.",
            "Baltic, Baltic states, Estonia, Estonian, Tallinn, Latvia, Latvian, Riga, Lithuania, Vilnius, and regional Baltic coverage.",
        ],

        // Middle East and Africa
        Region::MiddleEastNorthAfrica => &[
            "Israel, Israeli, Jerusalem, Tel Aviv, Gaza, Palestine, Palestinian, West Bank, Ramallah, Haifa, or Levantine coverage.",
            "Iran, Iranian, Tehran, Isfahan, Shiraz, Tabriz, Mashhad, Qom, Persian, Persian Gulf, or Iranian national news.",
            "Levant, Lebanon, Lebanese, Beirut, Tripoli Lebanon, Bekaa Valley, Syria, Damascus, Jordan, Amman, and nearby Arab states.",
            "Iraq, Iraqi, Baghdad, Basra, Mosul, Kurdistan, Erbil, Tigris, Euphrates, Mesopotamia, or Iraqi national news.",
            "Saudi Arabia, Saudi, Riyadh, Jeddah, Mecca, Medina, Neom, Hejaz, Najd, Saudi kingdom, or Gulf Arab news.",
            "United Arab Emirates, UAE, Emirati, Dubai, Abu Dhabi, Sharjah, Ajman, Fujairah, Ras Al Khaimah, Emirates, or UAE national news.",
            "Qatar, Qatari, Doha, Kuwait, Kuwaiti, Kuwait City, Gulf, Persian Gulf, Arabian Peninsula, Gulf states, and Gulf diplomacy.",
            "Egypt, Egyptian, Cairo, Alexandria, Giza, Sinai, Nile, Luxor, Aswan, Suez, or Egyptian national news.",
            "Morocco, Moroccan, Rabat, Casablanca, Marrakesh, Tangier, Fez, Atlas, Agadir, Morocco kingdom, or North African news.",
            "Algeria, Algerian, Algiers, Oran, Constantine, Sahara, Kabylie, Annaba, Tlemcen, Algerian, or Algerian national news.",
            "Tunisia, Tunisian, Tunis, Sfax, Carthage, Sousse, Kairouan, Djerba, Tunisia, Maghreb, or Tunisian national news.",
            "Libya, Libyan, Tripoli, Benghazi, Misrata, Sirte, Fezzan, Cyrenaica, North Africa, Maghreb, or Libyan national news.",
        ],
        Region::SubSaharanAfrica => &[
            "Nigeria, Nigerian, Lagos, Abuja, Kano, Kaduna, Rivers, Ibadan, Enugu, Port Harcourt, or Nigerian national news.",
            "Ghana, Ghanaian, Accra, Kumasi, Tamale, Cape Coast, Ashanti, Volta, Tema, West Africa, or Ghana national news.",
            "Senegal, Senegalese, Dakar, Saint-Louis, Casamance, Ivory Coast, Ivorian, Abidjan, West Africa, Sahel, or regional West Africa.",
            "Kenya, Kenyan, Nairobi, Mombasa, Kisumu, Nakuru, Rift Valley, East Africa, Kenyan, Kenya, or Kenyan national news.",
            "Ethiopia, Ethiopian, Addis Ababa, Amhara, Tigray, Oromo, Oromia, Gondar, East Africa, Horn of Africa, or Ethiopian national news.",
            "Uganda, Ugandan, Kampala, Entebbe, Jinja, Gulu, Lake Victoria, Buganda, East Africa, Uganda, or Ugandan national news.",
            "Tanzania, Tanzanian, Dar es Salaam, Zanzibar, Arusha, Dodoma, Serengeti, Kilimanjaro, East Africa, Tanzania, or Tanzanian national news.",
            "Rwanda, Rwandan, Kigali, Burundi, Burundian, Bujumbura, Great Lakes, East Africa, Rwanda, Burundi, or Great Lakes regional coverage.",
            "South Africa, South African, Johannesburg, Pretoria, Cape Town, Durban, Gauteng, Western Cape, Limpopo, Soweto, or South African national news.",
            "Zimbabwe, Zimbabwean, Harare, Bulawayo, Victoria Falls, Zambia, Zambian, Lusaka, Copperbelt, southern Africa, or southern African news.",
            "Botswana, Botswanan, Gaborone, Kalahari, Okavango, Namibia, Namibian, Windhoek, southern Africa, southern African, or Botswana national news.",
            "Congo, Democratic Republic of Congo, DRC, Congolese, Kinshasa, Lubumbashi, Kivu, Goma, central Africa, Congo River, or Congo basin news.",
            "Angola, Angolan, Luanda, Cameroon, Cameroonian, Yaounde, Douala, Gabon, Libreville, central Africa, or central African regional news.",
        ],

        // Asia
        Region::India => &[
            "India, Indian, New Delhi, Delhi, Mumbai, Kolkata, Chennai, Bengaluru, Hyderabad, India subcontinent, or Indian national news.",
            "India, Indian, Gujarat, Maharashtra, Kerala, Tamil Nadu, West Bengal, Rajasthan, Punjab, Karnataka, and major Indian states.",
            "India, Indian, Bihar, Uttar Pradesh, Madhya Pradesh, Odisha, Andhra Pradesh, Telangana, Haryana, and wider subcontinent coverage.",
            "Bangladesh, Bangladeshi, Dhaka, Chittagong, Sylhet, Khulna, Rajshahi, Cox's Bazar, Bengal, Padma, or Bangladeshi national news.",
            "Sri Lanka, Sri Lankan, Colombo, Kandy, Jaffna, Galle, Ceylon, Sinhalese, Tamil, Anuradhapura, or Sri Lankan national news.",
            "Maldives, Maldivian, Male, Addu, Hulhumale, Indian Ocean, atoll, Kaafu, Maafushi, Dhivehi, or Maldivian national news.",
            "Nepal, Nepalese, Kathmandu, Pokhara, Bhaktapur, Lalitpur, Himalayas, Kathmandu Valley, Himalayan, Nepal, or Nepal national news.",
            "Bhutan, Bhutanese, Thimphu, Paro, Punakha, Himalayas, Himalayan kingdom, Druk, Wangdue, Bhutan, or Bhutan national news.",
        ],
        Region::WestAsia => &[
            "Pakistan, Pakistani, Islamabad, Karachi, Lahore, Punjab, Sindh, Peshawar, Balochistan, Rawalpindi, or Pakistani national news.",
            "Pakistan, Pakistani, Karachi, Lahore, Punjab, Sindh, Khyber Pakhtunkhwa, Balochistan, Kashmir, Islamabad, and Pakistan politics.",
            "Afghanistan, Afghan, Kabul, Kandahar, Herat, Jalalabad, Pashtun, Hindu Kush, Mazar-i-Sharif, Balkh, or Afghan national news.",
            "Afghanistan, Afghan, Herat, Kunduz, Ghazni, Nangarhar, Helmand, Khyber Pass, Central Asia, West Asia, and Afghan regional coverage.",
        ],
        Region::China => &[
            "China, Chinese, Beijing, Shanghai, Shenzhen, Guangzhou, Wuhan, Sichuan, Guangdong, Zhejiang, or China national news.",
            "China, Chinese, People's Republic of China, Beijing, Shanghai, Guangdong, Sichuan, Zhejiang, Hubei, Xinjiang, and broader Chinese coverage.",
            "Hong Kong, Hongkonger, Kowloon, Macau, Macanese, Victoria Harbour, Hong Kong Island, New Territories, Lantau, Kowloon Peninsula, or Hong Kong and Macau stories.",
        ],
        Region::Japan => &[
            "Japan, Japanese, Tokyo, Osaka, Kyoto, Hokkaido, Okinawa, Yokohama, Nagoya, Hiroshima, or Japan national news.",
            "Japan, Japanese, Tokyo, Kansai, Honshu, Hokkaido, Kyushu, Shikoku, Sapporo, Fukuoka, and regional Japanese coverage.",
        ],
        Region::Korea => &[
            "Korea, Korean, North Korea, South Korea, Korean Peninsula, Seoul, Pyongyang, Busan, Incheon, DPRK, or Korean national news.",
            "Korea, Korean, North Korean, North Korea, Democratic People's Republic of Korea, Pyongyang, Kaesong, Hamhung, Juche, Korean Peninsula, and North Korean affairs.",
            "Korea, Korean, Republic of Korea, South Korean, Seoul, Gyeonggi, Daejeon, Jeju, Busan, Incheon, and South Korean coverage.",
        ],
        Region::Taiwan => &[
            "Taiwan, Taiwanese, Taipei, Kaohsiung, Taichung, Tainan, Formosa, Hsinchu, Taiwan Strait, Taiwanese, or Taiwan national news.",
            "Taiwan, Taiwanese, Taipei, New Taipei, Taoyuan, Hsinchu, Kaohsiung, Taichung, Tainan, Republic of China, and Taiwanese politics or business.",
        ],
        Region::SoutheastAsia => &[
            "Indonesia, Indonesian, Jakarta, Bali, Java, Sumatra, Sulawesi, Bandung, Surabaya, Borneo, or Indonesian national news.",
            "Vietnam, Vietnamese, Hanoi, Ho Chi Minh City, Saigon, Da Nang, Hue, Mekong, Haiphong, Vietnam, or Vietnamese national news.",
            "Thailand, Thai, Bangkok, Chiang Mai, Phuket, Pattaya, Isan, Ayutthaya, Krabi, Thailand, or Thai national news.",
            "Philippines, Filipino, Manila, Cebu, Luzon, Mindanao, Davao, Quezon City, Visayas, Tagalog, or Philippine national news.",
            "Malay Peninsula, Singapore, Singaporean, Malaysia, Malaysian, Kuala Lumpur, Penang, Johor, Sarawak, Sabah, or mainland Malay world coverage.",
            "Mainland Southeast Asia, Myanmar, Burmese, Yangon, Cambodia, Cambodian, Phnom Penh, Laos, Lao, Vientiane, and Mekong region news.",
            "Southeast Asia, ASEAN, South China Sea, Mekong, Indochina, Malacca Strait, maritime Southeast Asia, mainland Southeast Asia, and regional cross-border coverage.",
        ],
        Region::CentralAsia => &[
            "Kazakhstan, Kazakh, Astana, Almaty, Karaganda, Shymkent, Aktobe, Caspian, steppe, Central Asia, or Kazakh national news.",
            "Uzbekistan, Uzbek, Tashkent, Samarkand, Bukhara, Khiva, Fergana, Nukus, Andijan, Central Asia, or Uzbek national news.",
            "Kyrgyzstan, Kyrgyz, Bishkek, Osh, Issyk-Kul, Naryn, Jalal-Abad, Tien Shan, Central Asia, Kyrgyz Republic, or Kyrgyz national news.",
            "Tajikistan, Tajik, Dushanbe, Pamir, Khujand, Khorugh, Fann Mountains, Central Asia, Gorno-Badakhshan, Tajikistani, or Tajik national news.",
            "Turkmenistan, Turkmen, Ashgabat, Caspian Sea, Mary, Turkmenbashi, Karakum, Central Asia, Balkan Region, Turkmenistani, or Turkmen national news.",
            "Mongolia, Mongolian, Ulaanbaatar, Gobi, Steppe, Erdenet, Darkhan, Khovd, Mongol, Central Mongolia, or Mongolian national news.",
        ],

        // Oceania
        Region::Oceania => &[
            "Australia, Australian, Canberra, Sydney, Melbourne, Perth, Brisbane, Queensland, Tasmania, Western Australia, or Australian national news.",
            "New Zealand, New Zealander, Wellington, Auckland, Christchurch, Queenstown, North Island, South Island, Aotearoa, Kiwi, or New Zealand national news.",
            "Pacific islands, Fiji, Fijian, Samoa, Samoan, Tonga, Tongan, Papua New Guinea, Melanesia, Polynesia, and island state coverage.",
            "Oceania, Australasia, Pacific, South Pacific, Australia, New Zealand, Pacific Ocean, Pacific region, Australasian, and wider Pacific coverage.",
        ],
    }
}
