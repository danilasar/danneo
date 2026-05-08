/*
 * Control code javascript for Soft 
 * Project:      CMS Danneo : Content management system
 * File:         temp/Soft/javascript/soft.js
 * @copyright    Copyright (C) 2004 - 2013 Danneo Team. All rights reserved.
 * @link         http://danneo.com, http://danneo.ru
 * @license      http://www.gnu.org/licenses/gpl-2.0.html   GNU General Public License, version 2
 */

$('html').addClass('js');

$(function(){ 
    // меню 
    $('#menu').superfish(); 
    // подсказки при наведении 
    $("img, a, label").tooltip(); 
    // подсказки в поле 
    $("#search").textPlaceholder(); 
    // просмотр полного фото-видео 
    $(".media-view").colorbox({  
        scalePhotos   : 'true', 
        maxHeight     : '92%',
        maxWidth      : '94%'
    }); 
    // сообщение о недоступном 
    $.broken = function(url){
        window.location.href = url;
    }  
    // фотогалерея, полное фото-видео
    $("#image a").colorbox({  
        scalePhotos   : 'true', 
        maxHeight     : '92%',
        maxWidth      : '94%'
    }); 
    // фотогалерея, выделение кода в форме 
    $('.ccode').focus(function () { 
        $(this).select(); 
    }).mouseup(function(e){ 
        e.preventDefault(); 
    });  
}); 

$(window).bind('load', function() {
    $('#something').fadeTo(0, 0.75, function () {
        $('#something').fadeOut(200);
    });
    if ($.browser.msie && $.browser.version == 6){
        $(document.body).append('<iframe id="something" style="display: none"></iframe>');
    };
});
 
/**
 * @see http://github.com/NV/placeholder.js
 */
jQuery.fn.textPlaceholder = function () {
    return this.each(function(){
        var that = this;
        if (that.placeholder && 'placeholder' in document.createElement(that.tagName)) return;
        var placeholder = that.getAttribute('placeholder');
        var input = jQuery(that);
        if (that.value === '' || that.value == placeholder) {
	    input.addClass('text-placeholder');
            that.value = placeholder;
        }
        input.focus(function(){
            if (input.hasClass('text-placeholder')) {
	        this.value = '';
		input.removeClass('text-placeholder')
	    }
        });
        input.blur(function(){
            if (this.value === '') {
	        input.addClass('text-placeholder');
		this.value = placeholder;
	    } else {
	        input.removeClass('text-placeholder');
	    }
        });
        that.form && jQuery(that.form).submit(function(){
	    if (input.hasClass('text-placeholder')) {
	        that.value = '';
	   }
        });
    });
}; 

/**
 * Tool Tip 
 */
(function($){
     $.fn.tooltip = function(){
          var delay = 300, show, o = {xd : 15, yd : 19, top : 0, left : 0, x : 0, y : 0, mx : 0, my : 0, mc : 0};
          $('body').append('<div id="tooltip" style="display:none;position:absolute;"></div>');
          this.each(function(){
                var h = ($(this).is('img')) ? $(this).prop('alt') : $(this).prop('title');
                if(!$(this).hasClass('notooltip') && h.length > 0){
                $(this).bind('mouseover mousemove', function(e){
                    if (h) {
                        $('#tooltip').html(h);
                        ($(this).is("img")) ? $(this).prop('alt','') : $(this).prop('title','');
                        if ($('#tooltip').width() > 200) {
                        	$('#tooltip').css({'width':'200px'});
                        }
                        o.mc = document.getElementsByTagName((document.compatMode && document.compatMode == "CSS1Compat") ? "HTML":"BODY")[0];
                        o.left = e.pageX + o.xd;
                        o.top = e.pageY + o.yd;
                        o.mx = window.event ? event.clientX + o.mc.scrollLeft : e.pageX;
                        o.my = window.event ? event.clientY + o.mc.scrollTop : e.pageY;
                        if ((o.mx + $('#tooltip').width() + o.xd)  > (o.mc.clientWidth ? o.mc.clientWidth + o.mc.scrollLeft : window.innerWidth + window.pageXOffset) - 35) {
                            o.left = (o.mx - $('#tooltip').width() - 15);
                        }
                        if ((o.my + $('#tooltip').height() + o.yd) > (o.mc.innerHeight ? window.innerHeight + window.pageYOffset :o.mc.clientHeight + o.mc.scrollTop) - 50) {
                            o.top = (o.my - $('#tooltip').height() - 20);
                        }
                        if (e.type == 'mouseover') {
                            show = window.setTimeout(function() {
                                $('#tooltip').fadeIn(300).show().css({'top':o.top+'px','left':o.left+'px'});
                            }, delay);
                        }
                        if (e.type == 'mousemove') {
                            $('#tooltip').css({'top':o.top+'px','left':o.left+'px'});
                        }
                    }
               });
               $(this).mouseout(function(e){
                    $('#tooltip').hide();
                    $('#tooltip').css({'width':''});
                    window.clearTimeout(show);
                    ($(this).is("img")) ? $(this).prop('alt',h) : $(this).prop('title',h);
               }); 
               }
          });
     };
})(jQuery);
